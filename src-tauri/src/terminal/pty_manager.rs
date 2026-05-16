use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

use base64::Engine;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tauri::{AppHandle, Emitter};

use super::session::{PtySession, SendMaster};

/// PTY 终端管理器
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtySession>>,
}

impl PtyManager {
    /// 构造管理器
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// 创建终端会话
    pub fn create_session(
        &self,
        id: String,
        shell: Option<String>,
        app: AppHandle,
    ) -> Result<(), String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell_cmd = shell.unwrap_or_else(|| {
            if cfg!(windows) {
                "powershell.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        // 白名单校验防止注入
        validate_shell(&shell_cmd)?;

        let mut cmd = CommandBuilder::new(&shell_cmd);
        cmd.cwd(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")));

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        // 后台读取线程
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let session_id = id.clone();
        let reader_handle = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let engine = base64::engine::general_purpose::STANDARD;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // base64 编码后通过事件发送到前端
                        let encoded = engine.encode(&buf[..n]);
                        let event_name = format!("terminal-output:{}", session_id);
                        let _ = app.emit(&event_name, encoded);
                    }
                    Err(_) => break,
                }
            }
        });

        let session = PtySession {
            id: id.clone(),
            writer,
            child,
            master: SendMaster(pair.master),
            reader_handle: Some(reader_handle),
        };

        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        sessions.insert(id, session);
        Ok(())
    }

    /// 写入数据到终端
    pub fn write(&self, id: &str, data: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        let session = sessions.get_mut(id).ok_or("terminal not found")?;
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        session.writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 调整终端尺寸
    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        let session = sessions.get(id).ok_or("terminal not found")?;
        session
            .master
            .0
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 关闭终端
    pub fn close(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(mut session) = sessions.remove(id) {
            let _ = session.child.kill();
        }
        Ok(())
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 白名单校验 shell basename
fn validate_shell(shell: &str) -> Result<(), String> {
    const ALLOWED: &[&str] = &["cmd", "powershell", "pwsh", "bash", "sh", "zsh", "fish"];
    let path = std::path::Path::new(shell);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "无效的 shell 路径".to_string())?
        .to_lowercase();
    if !ALLOWED.contains(&stem.as_str()) {
        return Err(format!("不允许的 shell: {}", shell));
    }
    Ok(())
}
