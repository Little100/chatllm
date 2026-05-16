use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::sync::Notify;

/// 持久 PTY 会话
struct PersistentSession {
    writer: Box<dyn Write + Send>,
    #[allow(dead_code)]
    child: Box<dyn portable_pty::Child + Send + Sync>,
    buffer: Arc<Mutex<PtyBuffer>>,
    notify: Arc<Notify>,
    #[allow(dead_code)]
    reader_handle: std::thread::JoinHandle<()>,
}

struct PtyBuffer {
    data: String,
    last_update: Instant,
    closed: bool,
}

/// 快速模式的会话状态(仅保留 cwd)
struct QuickState {
    cwd: PathBuf,
}

/// AI 终端管理器
#[derive(Clone)]
pub struct AiShellManager {
    persistent: Arc<Mutex<HashMap<String, PersistentSession>>>,
    quick: Arc<Mutex<HashMap<String, QuickState>>>,
}

impl AiShellManager {
    pub fn new() -> Self {
        Self {
            persistent: Arc::new(Mutex::new(HashMap::new())),
            quick: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 统一入口
    #[allow(dead_code)]
    pub async fn exec_and_wait(
        &self,
        session_id: &str,
        input: &str,
        shell: &str,
        idle_timeout_secs: u64,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        // 向后兼容: 默认走快速模式
        self.exec_quick(session_id, input, shell, idle_timeout_secs, cancel_flag).await
    }

    /// 快速模式: 进程执行完即返回
    pub async fn exec_quick(
        &self,
        session_id: &str,
        input: &str,
        shell: &str,
        timeout_secs: u64,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        validate_shell(shell)?;
        let timeout = Duration::from_secs(timeout_secs.min(300));
        let trimmed = input.trim();

        let cwd = {
            let states = self.quick.lock().map_err(|e| e.to_string())?;
            states.get(session_id)
                .map(|s| s.cwd.clone())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| if cfg!(windows) { PathBuf::from("C:\\") } else { PathBuf::from("/") }))
        };

        let mut cmd = tokio::process::Command::new(shell_program(shell));
        apply_shell_args(&mut cmd, shell, trimmed);
        cmd.current_dir(&cwd);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("启动进程失败: {}", e))?;

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        let read_task = tokio::spawn(async move {
            let mut stdout_buf = Vec::new();
            let mut stderr_buf = Vec::new();
            if let Some(mut out) = stdout_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(&mut out, &mut stdout_buf).await;
            }
            if let Some(mut err) = stderr_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(&mut err, &mut stderr_buf).await;
            }
            (stdout_buf, stderr_buf)
        });

        let result = tokio::select! {
            r = tokio::time::timeout(timeout, child.wait()) => {
                match r {
                    Ok(Ok(status)) => {
                        let (stdout_buf, stderr_buf) = read_task.await.unwrap_or_default();
                        Ok((stdout_buf, stderr_buf, status.code().unwrap_or(-1)))
                    }
                    Ok(Err(e)) => Err(format!("进程执行失败: {}", e)),
                    Err(_) => {
                        let _ = child.kill().await;
                        Err("执行超时".to_string())
                    }
                }
            }
            _ = wait_cancel(cancel_flag) => {
                let _ = child.kill().await;
                Err("已打断".to_string())
            }
        };

        match result {
            Ok((stdout_buf, stderr_buf, exit_code)) => {
                let stdout = String::from_utf8_lossy(&stdout_buf).to_string();
                let stderr = String::from_utf8_lossy(&stderr_buf).to_string();
                self.try_update_cwd(session_id, trimmed, &cwd, shell).await;
                let formatted = format_output(&stdout, &stderr, exit_code);
                Ok(safe_truncate(&formatted, 50_000))
            }
            Err(e) => Err(e),
        }
    }

    /// 持久模式: PTY 会话保活, idle timeout 判定
    pub async fn exec_persistent(
        &self,
        name: &str,
        input: &str,
        shell: &str,
        idle_timeout_secs: u64,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        validate_shell(shell)?;

        let need_create = {
            let sessions = self.persistent.lock().map_err(|e| e.to_string())?;
            !sessions.contains_key(name)
        };
        if need_create {
            self.create_persistent(name, shell)?;
            self.wait_pty_ready(name).await?;
        }

        let (buffer_ref, notify_ref) = {
            let sessions = self.persistent.lock().map_err(|e| e.to_string())?;
            let s = sessions.get(name).ok_or("会话不存在")?;
            (s.buffer.clone(), s.notify.clone())
        };

        // 清空 buffer, 写入命令
        {
            let mut buf = buffer_ref.lock().map_err(|e| e.to_string())?;
            buf.data.clear();
            buf.last_update = Instant::now();
        }
        {
            let mut sessions = self.persistent.lock().map_err(|e| e.to_string())?;
            let s = sessions.get_mut(name).ok_or("会话不存在")?;
            let line = if input.ends_with('\n') { input.to_string() } else { format!("{}\r\n", input) };
            s.writer.write_all(line.as_bytes()).map_err(|e| format!("写入失败: {}", e))?;
            s.writer.flush().map_err(|e| format!("flush 失败: {}", e))?;
        }

        // idle timeout 等待
        let idle_dur = Duration::from_secs(idle_timeout_secs.max(2));
        let max_wait = Duration::from_secs(300);
        let start = Instant::now();

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            let (data, last_update, closed) = {
                let buf = buffer_ref.lock().map_err(|e| e.to_string())?;
                (buf.data.clone(), buf.last_update, buf.closed)
            };
            if closed { break; }
            if !data.is_empty() && last_update.elapsed() >= idle_dur { break; }
            if start.elapsed() >= max_wait { break; }

            let wait = idle_dur.min(Duration::from_millis(200));
            let _ = tokio::time::timeout(wait, notify_ref.notified()).await;
        }

        let output = {
            let mut buf = buffer_ref.lock().map_err(|e| e.to_string())?;
            let r = buf.data.clone();
            buf.data.clear();
            r
        };

        let cleaned = strip_ansi(&output);
        Ok(safe_truncate(&cleaned, 50_000))
    }

    /// 销毁持久会话
    pub fn destroy_session(&self, name: &str) -> Result<String, String> {
        let mut sessions = self.persistent.lock().map_err(|e| e.to_string())?;
        if let Some(mut s) = sessions.remove(name) {
            let _ = s.child.kill();
            Ok(format!("会话 '{}' 已销毁", name))
        } else {
            Err(format!("会话 '{}' 不存在", name))
        }
    }

    /// 列出活跃的持久会话
    #[allow(dead_code)]
    pub fn list_sessions(&self) -> Result<Vec<String>, String> {
        let sessions = self.persistent.lock().map_err(|e| e.to_string())?;
        Ok(sessions.keys().cloned().collect())
    }

    /// 关闭指定会话(兼容旧接口)
    #[allow(dead_code)]
    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        let _ = self.destroy_session(session_id);
        Ok(())
    }

    fn create_persistent(&self, name: &str, shell: &str) -> Result<(), String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;

        let shell_cmd = shell_program(shell);
        let mut cmd = CommandBuilder::new(shell_cmd);
        let is_powershell = matches!(shell, "powershell" | "pwsh");
        if is_powershell {
            cmd.arg("-NoLogo");
            cmd.arg("-NoProfile");
        }
        cmd.cwd(std::env::current_dir().unwrap_or_else(|_| if cfg!(windows) { PathBuf::from("C:\\") } else { PathBuf::from("/") }));

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let buffer = Arc::new(Mutex::new(PtyBuffer {
            data: String::new(),
            last_update: Instant::now(),
            closed: false,
        }));
        let notify = Arc::new(Notify::new());

        let buf_clone = buffer.clone();
        let ntf_clone = notify.clone();
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

        let reader_handle = std::thread::spawn(move || {
            let mut tmp = [0u8; 8192];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) => {
                        if let Ok(mut b) = buf_clone.lock() { b.closed = true; }
                        ntf_clone.notify_waiters();
                        break;
                    }
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&tmp[..n]);
                        if let Ok(mut b) = buf_clone.lock() {
                            b.data.push_str(&text);
                            b.last_update = Instant::now();
                        }
                        ntf_clone.notify_waiters();
                    }
                    Err(_) => {
                        if let Ok(mut b) = buf_clone.lock() { b.closed = true; }
                        ntf_clone.notify_waiters();
                        break;
                    }
                }
            }
        });

        let session = PersistentSession { writer, child, buffer, notify, reader_handle };
        let mut sessions = self.persistent.lock().map_err(|e| e.to_string())?;
        sessions.insert(name.to_string(), session);
        Ok(())
    }

    async fn wait_pty_ready(&self, name: &str) -> Result<(), String> {
        let (buffer_ref, notify_ref) = {
            let sessions = self.persistent.lock().map_err(|e| e.to_string())?;
            let s = sessions.get(name).ok_or("会话不存在")?;
            (s.buffer.clone(), s.notify.clone())
        };
        let deadline = Instant::now() + Duration::from_millis(2000);
        loop {
            let idle = {
                let buf = buffer_ref.lock().map_err(|e| e.to_string())?;
                if !buf.data.is_empty() { buf.last_update.elapsed() } else { Duration::ZERO }
            };
            if idle >= Duration::from_millis(300) {
                let mut buf = buffer_ref.lock().map_err(|e| e.to_string())?;
                buf.data.clear();
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let mut buf = buffer_ref.lock().map_err(|e| e.to_string())?;
                buf.data.clear();
                return Ok(());
            }
            let _ = tokio::time::timeout(remaining.min(Duration::from_millis(100)), notify_ref.notified()).await;
        }
    }

    async fn try_update_cwd(&self, session_id: &str, input: &str, current_cwd: &PathBuf, shell: &str) {
        if let Some(dir) = detect_cd_target(input, current_cwd, shell).await {
            if let Ok(mut states) = self.quick.lock() {
                states.entry(session_id.to_string())
                    .or_insert_with(|| QuickState { cwd: current_cwd.clone() })
                    .cwd = dir;
            }
        }
    }
}

impl Default for AiShellManager {
    fn default() -> Self {
        Self::new()
    }
}

async fn wait_cancel(flag: &Arc<AtomicBool>) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn format_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
    let mut out = String::new();
    if !stdout.is_empty() {
        out.push_str(stdout.trim_end());
    }
    if !stderr.is_empty() {
        if !out.is_empty() { out.push('\n'); }
        out.push_str("[stderr] ");
        out.push_str(stderr.trim_end());
    }
    if exit_code != 0 {
        if !out.is_empty() { out.push('\n'); }
        out.push_str(&format!("(退出码: {})", exit_code));
    }
    if out.is_empty() {
        out.push_str("(无输出)");
    }
    out
}

fn shell_program(shell: &str) -> &str {
    match shell {
        "cmd" => "cmd.exe",
        "pwsh" => "pwsh.exe",
        "powershell" => "powershell.exe",
        "bash" => "bash",
        "sh" => "sh",
        "zsh" => "zsh",
        "fish" => "fish",
        _ => {
            if cfg!(windows) { "powershell.exe" } else { "bash" }
        }
    }
}

fn apply_shell_args(cmd: &mut tokio::process::Command, shell: &str, input: &str) {
    match shell {
        "cmd" => {
            cmd.args(["/C", &format!("chcp 65001 >nul & {}", input)]);
        }
        "bash" | "sh" | "zsh" | "fish" => {
            cmd.args(["-c", input]);
        }
        _ => {
            cmd.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
                &format!("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}", input)]);
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ('\x40'..='\x7e').contains(&ch) { break; }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '\x07' { break; }
                        if ch == '\x1b' {
                            if chars.peek() == Some(&'\\') { chars.next(); }
                            break;
                        }
                    }
                }
                _ => { chars.next(); }
            }
        } else {
            out.push(c);
        }
    }
    out
}

async fn detect_cd_target(input: &str, current_cwd: &PathBuf, shell: &str) -> Option<PathBuf> {
    let target = match shell {
        "cmd" => extract_cd_target_cmd(input.trim())?,
        "bash" | "sh" | "zsh" | "fish" => extract_cd_target_unix(input.trim())?,
        _ => extract_cd_target_ps(input.trim())?,
    };
    let path = if std::path::Path::new(&target).is_absolute() {
        PathBuf::from(&target)
    } else {
        current_cwd.join(&target)
    };
    match tokio::fs::canonicalize(&path).await {
        Ok(canonical) if canonical.is_dir() => Some(canonical),
        _ => None,
    }
}

fn extract_cd_target_cmd(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    if lower.starts_with("cd ") || lower.starts_with("cd\\") || lower.starts_with("cd/") {
        Some(input[2..].trim().trim_matches('"').to_string())
    } else {
        None
    }
}

fn extract_cd_target_unix(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed == "cd" || trimmed == "cd ~" {
        return Some("~".to_string());
    }
    if trimmed.starts_with("cd ") {
        let target = trimmed[3..].trim().trim_matches('"').trim_matches('\'');
        return Some(target.to_string());
    }
    None
}

fn extract_cd_target_ps(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    for prefix in ["cd ", "set-location ", "sl ", "chdir "] {
        if lower.starts_with(prefix) {
            let rest = input[prefix.len()..].trim();
            let target = if rest.to_lowercase().starts_with("-path ") { rest[6..].trim() } else { rest };
            return Some(target.trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

fn validate_shell(shell: &str) -> Result<(), String> {
    const ALLOWED: &[&str] = &["cmd", "powershell", "pwsh", "bash", "sh", "zsh", "fish"];
    if ALLOWED.contains(&shell.to_lowercase().as_str()) {
        Ok(())
    } else {
        Err(format!("不允许的 shell: {}", shell))
    }
}

fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut = s.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(s.len());
    format!("{}...\n(输出过长, 已截断)", &s[..cut])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_output_basic() {
        assert_eq!(format_output("hello\n", "", 0), "hello");
    }

    #[test]
    fn format_output_with_stderr() {
        let out = format_output("ok\n", "warn\n", 0);
        assert!(out.contains("[stderr] warn"));
    }

    #[test]
    fn format_output_nonzero_exit() {
        let out = format_output("", "err\n", 1);
        assert!(out.contains("(退出码: 1)"));
    }

    #[test]
    fn extract_cd_ps_basic() {
        assert_eq!(extract_cd_target_ps("cd src"), Some("src".to_string()));
        assert_eq!(extract_cd_target_ps("Set-Location C:\\Users"), Some("C:\\Users".to_string()));
    }

    #[test]
    fn extract_cd_cmd_basic() {
        assert_eq!(extract_cd_target_cmd("cd src"), Some("src".to_string()));
    }

    #[test]
    fn validate_shell_rejects_unknown() {
        assert!(validate_shell("ruby").is_err());
        assert!(validate_shell("powershell").is_ok());
        assert!(validate_shell("bash").is_ok());
        assert!(validate_shell("zsh").is_ok());
    }

    #[test]
    fn strip_ansi_works() {
        assert_eq!(strip_ansi("\x1b[32mhi\x1b[0m"), "hi");
    }
}
