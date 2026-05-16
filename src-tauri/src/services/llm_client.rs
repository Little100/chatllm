use serde_json::{json, Value};

use crate::models::ChatMessage;

/// 剥离 user_attachments marker, 避免发送给 LLM
fn strip_user_attachments_marker(content: &str) -> &str {
    if let Some(pos) = content.find("\n\n<!-- user_attachments:") {
        &content[..pos]
    } else {
        content
    }
}

/// 剥离 chat_rounds marker, 避免工具调用历史污染 LLM 上下文
fn strip_chat_rounds_marker(content: &str) -> &str {
    if let Some(pos) = content.find("\n\n<!-- chat_rounds:") {
        &content[..pos]
    } else if let Some(pos) = content.find("<!-- chat_rounds:") {
        &content[..pos]
    } else {
        content
    }
}

/// 从 chat_rounds marker 中提取工具执行摘要, 供历史上下文使用
fn extract_rounds_summary(content: &str) -> String {
    let marker_start = if let Some(pos) = content.find("<!-- chat_rounds:") {
        pos + "<!-- chat_rounds:".len()
    } else {
        return "(已完成)".to_string();
    };
    let marker_end = content[marker_start..].find(" -->")
        .map(|p| marker_start + p)
        .unwrap_or(content.len());
    let json_str = &content[marker_start..marker_end];

    // 尝试解析 rounds JSON, 提取工具名和结果摘要
    if let Ok(rounds) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        let mut parts: Vec<String> = Vec::new();
        for round in &rounds {
            if let Some(execs) = round.get("tool_execs").and_then(|v| v.as_array()) {
                for exec in execs {
                    let name = exec.get("name").and_then(|v| v.as_str()).unwrap_or("tool");
                    let preview = exec.get("result_preview").and_then(|v| v.as_str()).unwrap_or("");
                    let short: String = preview.chars().take(200).collect();
                    parts.push(format!("[{}]: {}", name, short));
                }
            }
            if let Some(text) = round.get("content").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    parts.push(text.chars().take(300).collect());
                }
            }
        }
        if parts.is_empty() {
            "(已完成)".to_string()
        } else {
            parts.join("\n")
        }
    } else {
        "(已完成)".to_string()
    }
}

/// LLM HTTP 客户端
pub struct LlmClient {
    client: reqwest::Client,
}

impl LlmClient {
    /// 构建客户端
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .read_timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");
        Self { client }
    }

    /// 构造 OpenAI 兼容请求体
    #[allow(dead_code)]
    pub fn build_request_body(
        model: &str,
        messages: &[ChatMessage],
        max_tokens: i32,
        temperature: f64,
        top_p: f64,
        system_prompt: Option<&str>,
        stream: bool,
    ) -> Value {
        Self::build_request_body_full(
            model, messages, max_tokens, temperature, top_p, true, true, system_prompt, stream, None, None, false,
        )
    }

    /// 把 ChatMessage 转 OpenAI 消息 JSON, assistant 角色可选附带 reasoning_content
    pub fn message_to_json(m: &ChatMessage, passback_reasoning: bool) -> Value {
        let content = strip_user_attachments_marker(&m.content);
        let clean_content = strip_chat_rounds_marker(content);
        // 如果 assistant 消息 strip 后为空(纯工具调用轮次), 用摘要替代防止历史断裂
        let final_content = if clean_content.trim().is_empty() && m.role == "assistant" {
            extract_rounds_summary(&m.content)
        } else {
            clean_content.to_string()
        };
        let mut obj = json!({"role": m.role, "content": final_content});
        if passback_reasoning && m.role == "assistant" {
            if let Some(t) = &m.thinking {
                if !t.is_empty() {
                    obj["reasoning_content"] = json!(t);
                }
            }
        }
        obj
    }

    /// 构造请求体(完整版，支持推理强度和参数开关)
    #[allow(clippy::too_many_arguments)]
    pub fn build_request_body_full(
        model: &str,
        messages: &[ChatMessage],
        max_tokens: i32,
        temperature: f64,
        top_p: f64,
        temperature_enabled: bool,
        top_p_enabled: bool,
        system_prompt: Option<&str>,
        stream: bool,
        tools: Option<&[Value]>,
        reasoning_effort: Option<&str>,
        passback_reasoning: bool,
    ) -> Value {
        let mut msg_array: Vec<Value> = Vec::new();
        if let Some(sys) = system_prompt {
            if !sys.is_empty() {
                msg_array.push(json!({
                    "role": "system",
                    "content": sys,
                }));
            }
        }
        for m in messages {
            msg_array.push(Self::message_to_json(m, passback_reasoning));
        }
        let mut body = json!({
            "model": model,
            "messages": msg_array,
            "max_tokens": max_tokens,
            "stream": stream,
        });
        if temperature_enabled {
            body["temperature"] = json!(temperature);
        }
        if top_p_enabled {
            body["top_p"] = json!(top_p);
        }
        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
            }
        }
        if let Some(effort) = reasoning_effort {
            if !effort.is_empty() && effort != "none" {
                body["reasoning_effort"] = json!(effort);
            }
        }
        body
    }

    /// 构造带原始消息数组的请求体(用于 tool calling 循环中追加 tool 消息)
    #[allow(clippy::too_many_arguments)]
    pub fn build_request_body_raw(
        model: &str,
        messages: &[Value],
        max_tokens: i32,
        temperature: f64,
        top_p: f64,
        temperature_enabled: bool,
        top_p_enabled: bool,
        stream: bool,
        tools: Option<&[Value]>,
        reasoning_effort: Option<&str>,
    ) -> Value {
        let mut body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "stream": stream,
        });
        if temperature_enabled {
            body["temperature"] = json!(temperature);
        }
        if top_p_enabled {
            body["top_p"] = json!(top_p);
        }
        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
            }
        }
        if let Some(effort) = reasoning_effort {
            if !effort.is_empty() && effort != "none" {
                body["reasoning_effort"] = json!(effort);
            }
        }
        body
    }

    /// 发送流式聊天请求
    pub async fn send_chat_stream(
        &self,
        base_url: &str,
        api_key: &str,
        body: Value,
    ) -> Result<reqwest::Response, String> {
        let url = Self::build_chat_url(base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp)
    }

    /// 发送非流式聊天请求
    pub async fn send_chat(
        &self,
        base_url: &str,
        api_key: &str,
        body: Value,
    ) -> Result<Value, String> {
        let url = Self::build_chat_url(base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, text));
        }

        resp.json::<Value>().await.map_err(|e| e.to_string())
    }

    /// 拼接补全端点
    fn build_chat_url(base_url: &str) -> String {
        let trimmed = base_url.trim_end_matches('/');
        if trimmed.ends_with("/chat/completions") {
            trimmed.to_string()
        } else if trimmed.ends_with("/v1") {
            format!("{}/chat/completions", trimmed)
        } else {
            format!("{}/v1/chat/completions", trimmed)
        }
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}
