use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::AppHandle;

use crate::terminal::ai_shell::AiShellManager;

/// 工具参数属性定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// 工具参数 schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, ToolProperty>,
    pub required: Vec<String>,
}

/// 工具定义(发送给 LLM 的 schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
    pub success: bool,
}

/// 从 LLM 响应中解析出的完整工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 工具注册表，管理可用工具和执行
pub struct ToolRegistry {
    definitions: Vec<ToolDefinition>,
}

impl ToolRegistry {
    /// 构建内置工具注册表
    pub fn new() -> Self {
        let mut registry = Self { definitions: Vec::new() };
        registry.register_builtin_tools();
        registry
    }

    /// 获取所有工具的 schema(用于 LLM 请求)
    pub fn get_schemas(&self) -> Vec<Value> {
        self.definitions.iter().map(|def| {
            json!({
                "type": "function",
                "function": {
                    "name": def.name,
                    "description": def.description,
                    "parameters": {
                        "type": "object",
                        "properties": def.parameters.properties.iter().map(|(k, v)| {
                            let mut prop = json!({
                                "type": v.prop_type,
                                "description": v.description,
                            });
                            if let Some(ref enums) = v.enum_values {
                                prop["enum"] = json!(enums);
                            }
                            (k.clone(), prop)
                        }).collect::<serde_json::Map<String, Value>>(),
                        "required": def.parameters.required,
                    }
                }
            })
        }).collect()
    }

    /// 执行工具调用, session_id 用于隔离 PTY, cancel_flag 用于打断 shell 等待
    pub async fn execute(
        &self,
        tool_call: &ToolCall,
        db: &SqlitePool,
        ai_shell: &AiShellManager,
        app: &AppHandle,
        session_id: &str,
        cancel_flag: &Arc<AtomicBool>,
    ) -> ToolResult {
        let result = match tool_call.name.as_str() {
            "web_search" => self.exec_web_search(tool_call, db).await,
            "read_file" => self.exec_read_file(tool_call, app).await,
            "write_file" => self.exec_write_file(tool_call, app).await,
            "str_replace" => self.exec_str_replace(tool_call).await,
            "url_fetch" => self.exec_url_fetch(tool_call).await,
            "shell_exec" => self.exec_shell(tool_call, ai_shell, session_id, cancel_flag).await,
            "shell_session_destroy" => self.exec_shell_destroy(tool_call, ai_shell).await,
            _ => Err(format!("未知工具: {}", tool_call.name)),
        };

        match result {
            Ok(content) => ToolResult {
                tool_call_id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                content,
                success: true,
            },
            Err(err) => ToolResult {
                tool_call_id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                content: format!("执行失败: {}", err),
                success: false,
            },
        }
    }

    fn register_builtin_tools(&mut self) {
        self.definitions.push(Self::def_web_search());
        self.definitions.push(Self::def_read_file());
        self.definitions.push(Self::def_write_file());
        self.definitions.push(Self::def_str_replace());
        self.definitions.push(Self::def_url_fetch());
        self.definitions.push(Self::def_shell_exec());
        self.definitions.push(Self::def_shell_session_destroy());
    }

    fn def_web_search() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("query".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "搜索关键词".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "web_search".to_string(),
            description: "联网搜索获取实时信息，返回搜索结果摘要".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["query".to_string()],
            },
        }
    }

    fn def_read_file() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("path".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "文件的绝对路径".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "read_file".to_string(),
            description: "读取指定路径的文件内容".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["path".to_string()],
            },
        }
    }

    fn def_write_file() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("path".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "文件的绝对路径".to_string(),
            enum_values: None,
        });
        props.insert("content".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "要写入的内容".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "write_file".to_string(),
            description: "写入内容到指定路径的文件".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["path".to_string(), "content".to_string()],
            },
        }
    }

    fn def_str_replace() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("path".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "文件的绝对路径".to_string(),
            enum_values: None,
        });
        props.insert("old_str".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "要被替换的原始文本片段(必须在文件中唯一匹配)".to_string(),
            enum_values: None,
        });
        props.insert("new_str".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "替换后的新文本".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "str_replace".to_string(),
            description: "在文件中精确替换一段文本。old_str 必须在文件内唯一出现，将被替换为 new_str。适合小范围编辑，比 write_file 重写整个文件更安全。".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["path".to_string(), "old_str".to_string(), "new_str".to_string()],
            },
        }
    }

    fn def_url_fetch() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("url".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "要抓取的网页 URL(仅 http/https, 禁止内网地址)".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "url_fetch".to_string(),
            description: "抓取指定 URL 的网页内容，返回纯文本".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["url".to_string()],
            },
        }
    }

    fn def_shell_exec() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("command".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "要执行的命令".to_string(),
            enum_values: None,
        });
        props.insert("shell".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "shell 类型: Windows 下可选 cmd/powershell/pwsh, macOS/Linux 下可选 bash/sh/zsh(默认根据系统自动选择)".to_string(),
            enum_values: Some(vec!["cmd".to_string(), "powershell".to_string(), "pwsh".to_string(), "bash".to_string(), "sh".to_string(), "zsh".to_string()]),
        });
        props.insert("timeout_secs".to_string(), ToolProperty {
            prop_type: "integer".to_string(),
            description: "超时秒数(快速模式为进程超时, 持久模式为空闲超时, 默认 30, 最大 300)".to_string(),
            enum_values: None,
        });
        props.insert("session_name".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "持久会话名称。提供此参数时使用 PTY 保活模式(适用于 SSH、REPL 等交互式场景), 不提供则使用快速模式(命令结束即返回)".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "shell_exec".to_string(),
            description: "执行终端命令。默认快速模式: 命令结束立即返回结果。若需保活终端(SSH/REPL等), 传入 session_name 开启持久模式, 通过 idle 超时返回输出。持久会话不用时应调用 shell_session_destroy 销毁。".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["command".to_string()],
            },
        }
    }

    fn def_shell_session_destroy() -> ToolDefinition {
        let mut props = HashMap::new();
        props.insert("session_name".to_string(), ToolProperty {
            prop_type: "string".to_string(),
            description: "要销毁的持久会话名称".to_string(),
            enum_values: None,
        });
        ToolDefinition {
            name: "shell_session_destroy".to_string(),
            description: "销毁指定的持久终端会话, 释放资源。不再需要 SSH/REPL 连接时调用。".to_string(),
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties: props,
                required: vec!["session_name".to_string()],
            },
        }
    }

    async fn exec_web_search(&self, tool_call: &ToolCall, db: &SqlitePool) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let query = args["query"].as_str().ok_or("缺少 query 参数")?;

        let row = sqlx::query(
            "SELECT * FROM search_configs WHERE enabled = 1 LIMIT 1"
        )
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?;

        let row = row.ok_or_else(|| "未配置搜索模型".to_string())?;

        use sqlx::Row;
        let prompt_template: String = row.get("prompt_template");
        let model: String = row.get("model");
        let base_url: Option<String> = row.get("base_url");
        let api_key_name: Option<String> = row.get("api_key_name");
        let api_key_db: Option<String> = row.try_get("api_key").unwrap_or(None);
        let max_tokens: i32 = row.get("max_tokens");
        let temperature: f64 = row.get("temperature");
        let top_p: f64 = row.get("top_p");
        let temperature_enabled: i32 = row.try_get("temperature_enabled").unwrap_or(1);
        let top_p_enabled: i32 = row.try_get("top_p_enabled").unwrap_or(1);
        let reasoning_effort: Option<String> = row.try_get("reasoning_effort").unwrap_or(None);

        let base_url = base_url.ok_or_else(|| "搜索模型未配置接口地址".to_string())?;

        let api_key = crate::commands::web_search::resolve_api_key(
            api_key_name.as_deref(),
            api_key_db.as_deref(),
        )?;

        let prompt = prompt_template.replace("{query}", query);
        let mut body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "stream": false,
        });
        if temperature_enabled == 1 {
            body["temperature"] = json!(temperature);
        }
        if top_p_enabled == 1 {
            body["top_p"] = json!(top_p);
        }
        if let Some(effort) = reasoning_effort.as_deref() {
            if !effort.is_empty() && effort != "none" {
                body["reasoning_effort"] = json!(effort);
            }
        }

        let client = crate::services::llm_client::LlmClient::new();
        let future = client.send_chat(&base_url, &api_key, body);
        let resp = tokio::time::timeout(std::time::Duration::from_secs(60), future)
            .await
            .map_err(|_| "搜索请求超时(60秒)".to_string())?
            .map_err(|e| e.to_string())?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err("搜索未返回有效内容".to_string());
        }
        Ok(content)
    }

    async fn exec_read_file(&self, tool_call: &ToolCall, _app: &AppHandle) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let path = args["path"].as_str().ok_or("缺少 path 参数")?;

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("读取失败: {}", e))?;

        if content.chars().count() > 50_000 {
            let cut = char_boundary(&content, 50_000);
            Ok(format!(
                "{}...\n\n(内容过长，已截断，共 {} 字节)",
                &content[..cut],
                content.len()
            ))
        } else {
            Ok(content)
        }
    }

    async fn exec_write_file(&self, tool_call: &ToolCall, _app: &AppHandle) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let path = args["path"].as_str().ok_or("缺少 path 参数")?;
        let content = args["content"].as_str().ok_or("缺少 content 参数")?;

        let target = std::path::Path::new(path);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }

        tokio::fs::write(target, content.as_bytes())
            .await
            .map_err(|e| format!("写入失败: {}", e))?;

        Ok(format!("已写入 {} 字节到 {}", content.len(), target.display()))
    }

    async fn exec_str_replace(&self, tool_call: &ToolCall) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let path = args["path"].as_str().ok_or("缺少 path 参数")?;
        let old_str = args["old_str"].as_str().ok_or("缺少 old_str 参数")?;
        let new_str = args["new_str"].as_str().ok_or("缺少 new_str 参数")?;

        if old_str.is_empty() {
            return Err("old_str 不能为空".to_string());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("读取失败: {}", e))?;

        let count = content.matches(old_str).count();
        if count == 0 {
            return Err("old_str 在文件中未找到匹配".to_string());
        }
        if count > 1 {
            return Err(format!("old_str 在文件中匹配了 {} 处, 必须唯一", count));
        }

        let new_content = content.replacen(old_str, new_str, 1);
        tokio::fs::write(path, new_content.as_bytes())
            .await
            .map_err(|e| format!("写入失败: {}", e))?;

        Ok(format!("已替换 {} 中的文本片段", path))
    }

    async fn exec_url_fetch(&self, tool_call: &ToolCall) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let url_str = args["url"].as_str().ok_or("缺少 url 参数")?;

        // SSRF 防护
        validate_public_url(url_str)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client.get(url_str)
            .header("User-Agent", "ChatLLM/1.0")
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let text = resp.text().await.map_err(|e| e.to_string())?;

        let clean = strip_html_tags(&text);

        if clean.chars().count() > 50_000 {
            let cut = char_boundary(&clean, 50_000);
            Ok(format!("{}...\n\n(内容过长，已截断)", &clean[..cut]))
        } else {
            Ok(clean)
        }
    }

    async fn exec_shell(
        &self,
        tool_call: &ToolCall,
        ai_shell: &AiShellManager,
        session_id: &str,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let command = args["command"].as_str().ok_or("缺少 command 参数")?;
        let shell = args["shell"].as_str().unwrap_or("powershell");
        let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(30).min(300);
        let session_name = args["session_name"].as_str();

        match session_name {
            Some(name) => {
                // 持久模式: PTY 保活
                ai_shell
                    .exec_persistent(name, command, shell, timeout_secs, cancel_flag)
                    .await
            }
            None => {
                // 快速模式: 进程结束即返回
                let quick_id = format!("quick_{}", session_id);
                ai_shell
                    .exec_quick(&quick_id, command, shell, timeout_secs, cancel_flag)
                    .await
            }
        }
    }

    async fn exec_shell_destroy(
        &self,
        tool_call: &ToolCall,
        ai_shell: &AiShellManager,
    ) -> Result<String, String> {
        let args: Value = serde_json::from_str(&tool_call.arguments)
            .map_err(|e| format!("参数解析失败: {}", e))?;
        let name = args["session_name"].as_str().ok_or("缺少 session_name 参数")?;
        ai_shell.destroy_session(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 取字符索引到字节偏移, 不足则返回末尾
fn char_boundary(s: &str, max_chars: usize) -> usize {
    s.char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// 字节级 HTML 标签剥离, 避免 UTF-8 切片越界
fn strip_html_tags(html: &str) -> String {
    let bytes = html.as_bytes();
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let mut result_bytes: Vec<u8> = Vec::with_capacity(bytes.len());

    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let mut i = 0usize;
    while i < bytes.len() {
        if !in_tag && !in_script && !in_style && lower[i..].starts_with(b"<script") {
            in_script = true;
            in_tag = true;
            i += 1;
            continue;
        }
        if !in_tag && !in_script && !in_style && lower[i..].starts_with(b"<style") {
            in_style = true;
            in_tag = true;
            i += 1;
            continue;
        }
        if in_script && lower[i..].starts_with(b"</script>") {
            in_script = false;
            in_tag = false;
            i += 9;
            continue;
        }
        if in_style && lower[i..].starts_with(b"</style>") {
            in_style = false;
            in_tag = false;
            i += 8;
            continue;
        }
        let b = bytes[i];
        if b == b'<' {
            in_tag = true;
            i += 1;
            continue;
        }
        if b == b'>' && in_tag && !in_script && !in_style {
            in_tag = false;
            i += 1;
            continue;
        }

        if !in_tag && !in_script && !in_style {
            result_bytes.push(b);
        }
        i += 1;
    }

    let result = String::from_utf8_lossy(&result_bytes).into_owned();

    let mut compressed = String::with_capacity(result.len());
    let mut prev_whitespace = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !prev_whitespace {
                compressed.push(' ');
            }
            prev_whitespace = true;
        } else {
            compressed.push(ch);
            prev_whitespace = false;
        }
    }
    compressed.trim().to_string()
}

/// 仅放行公网 http/https URL, 拒绝内网与回环
fn validate_public_url(url_str: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(url_str).map_err(|e| format!("URL 解析失败: {}", e))?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("仅允许 http/https, 实际: {}", scheme));
    }
    let host = url.host_str().ok_or_else(|| "URL 缺少 host".to_string())?;
    let host_lower = host.to_lowercase();
    if matches!(host_lower.as_str(), "localhost" | "0.0.0.0" | "ip6-localhost" | "ip6-loopback") {
        return Err(format!("禁止访问本机地址: {}", host));
    }
    // 主机若是字面 IP 才做范围校验, 域名不阻塞
    if let Ok(ip) = host_lower.parse::<std::net::IpAddr>() {
        if is_disallowed_ip(&ip) {
            return Err(format!("禁止访问内网地址: {}", host));
        }
    }
    Ok(())
}

/// 判定 IP 是否落在禁止段
fn is_disallowed_ip(ip: &std::net::IpAddr) -> bool {
    if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
        return true;
    }
    match ip {
        std::net::IpAddr::V4(v4) => {
            // 私网与链路本地, 手动判定避免依赖不稳定 API
            if v4.is_private() || v4.is_link_local() || v4.is_broadcast() {
                return true;
            }
            let oc = v4.octets();
            // CGNAT 100.64.0.0/10
            if oc[0] == 100 && (oc[1] & 0xC0) == 64 {
                return true;
            }
            // 169.254/16 已被 is_link_local 覆盖
            false
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_multicast() || v6.is_unspecified() {
                return true;
            }
            // ULA fc00::/7
            let seg = v6.segments();
            if (seg[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // 链路本地 fe80::/10
            if (seg[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_handles_non_ascii() {
        // 含中文与脚本/样式块, 不能 panic
        let html = "<html><head><style>body{}</style></head><body><p>你好<b>世界</b></p><script>x=1</script></body></html>";
        let out = strip_html_tags(html);
        assert!(out.contains("你好"));
        assert!(out.contains("世界"));
        assert!(!out.contains("x=1"));
    }

    #[test]
    fn ssrf_blocks_localhost() {
        assert!(validate_public_url("http://localhost/").is_err());
        assert!(validate_public_url("http://127.0.0.1/").is_err());
        assert!(validate_public_url("http://10.0.0.1/").is_err());
        assert!(validate_public_url("http://192.168.1.1/").is_err());
        assert!(validate_public_url("ftp://example.com/").is_err());
        assert!(validate_public_url("https://example.com/").is_ok());
    }
}
