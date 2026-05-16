use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::{Manager, Emitter};
use futures_util::StreamExt;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::models::{ChatMessage, ModelConfig, StreamEvent};
use crate::services::llm_client::LlmClient;
use crate::services::sse_parser::{ParsedEvent, SseParser};
use crate::services::stream_manager::StreamManager;
use crate::services::tools::{ToolCall, ToolRegistry};

/// 工具循环硬超时
const TOOL_LOOP_TIMEOUT_SECS: u64 = 300;
/// 节流保存阈值: token 累计数
const PERSIST_TOKEN_THRESHOLD: usize = 64;
/// 节流保存阈值: 时间间隔
const PERSIST_INTERVAL: Duration = Duration::from_millis(500);

/// 单轮流式响应的累积结果
struct RoundResult {
    content: String,
    thinking: String,
    tool_calls: Vec<ToolCall>,
}

/// 轮次内单个工具调用的快照(持久化用)
#[derive(Debug, Clone, serde::Serialize)]
struct RoundToolCallSnapshot {
    name: String,
    arguments: String,
}

/// 轮次内单个工具执行结果的快照(持久化用)
#[derive(Debug, Clone, serde::Serialize)]
struct RoundToolExecSnapshot {
    name: String,
    arguments: String,
    success: bool,
    result_preview: String,
}

/// 单轮交互的完整快照(写入 marker JSON)
#[derive(Debug, Clone, serde::Serialize)]
struct RoundSnapshot {
    thinking: String,
    content: String,
    tool_calls: Vec<RoundToolCallSnapshot>,
    tool_execs: Vec<RoundToolExecSnapshot>,
}

/// 等待 cancel 标记被置位的辅助 future, 用于与 stream 竞争
async fn wait_cancel(flag: &Arc<AtomicBool>) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// 剥离 chat_rounds marker, 避免工具调用历史污染后续请求
fn strip_rounds_marker(content: &str) -> &str {
    if let Some(pos) = content.find("\n\n<!-- chat_rounds:") {
        &content[..pos]
    } else if let Some(pos) = content.find("<!-- chat_rounds:") {
        &content[..pos]
    } else {
        content
    }
}

/// 节流持久化当前累积内容
async fn persist_partial(
    db: &SqlitePool,
    msg_id: &str,
    content_prefix: &str,
    full_content: &str,
    thinking_prefix: &str,
    full_thinking: &str,
) {
    let combined = if content_prefix.is_empty() {
        full_content.to_string()
    } else if full_content.is_empty() {
        content_prefix.to_string()
    } else {
        format!("{}\n\n{}", content_prefix, full_content)
    };
    let thinking_combined = if thinking_prefix.is_empty() {
        full_thinking.to_string()
    } else {
        format!("{}{}", thinking_prefix, full_thinking)
    };
    let thinking_arg = if thinking_combined.is_empty() {
        None
    } else {
        Some(thinking_combined)
    };
    let _ = sqlx::query("UPDATE messages SET content = ?, thinking = ? WHERE id = ?")
        .bind(&combined)
        .bind(&thinking_arg)
        .bind(msg_id)
        .execute(db)
        .await;
}

/// 消费一个流式响应，发送 token/thinking/tool_call 事件，累积工具调用并节流落库
#[allow(clippy::too_many_arguments)]
async fn consume_stream(
    response: reqwest::Response,
    channel: &Channel<StreamEvent>,
    msg_id: &str,
    cancel_flag: &Arc<AtomicBool>,
    db: &SqlitePool,
    content_prefix: &str,
    thinking_prefix: &str,
) -> RoundResult {
    let mut parser = SseParser::new();
    let mut full_content = String::new();
    let mut full_thinking = String::new();
    let mut accum: Vec<(usize, Option<String>, String, String)> = Vec::new();
    let mut stream = response.bytes_stream();
    let mut stream_done = false;
    let mut tokens_since_save: usize = 0;
    let mut last_save = Instant::now();

    while !stream_done {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }
        // 与取消标记竞争, 让取消能在 50ms 内生效
        let chunk_result = tokio::select! {
            r = stream.next() => r,
            _ = wait_cancel(cancel_flag) => None,
        };
        let chunk_result = match chunk_result {
            Some(r) => r,
            None => break,
        };
        match chunk_result {
            Ok(bytes) => {
                let events = parser.feed(&bytes);
                for event in events {
                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    match event {
                        ParsedEvent::Token(text) => {
                            full_content.push_str(&text);
                            tokens_since_save += 1;
                            let _ = channel.send(StreamEvent::Token { content: text });
                        }
                        ParsedEvent::Thinking(text) => {
                            full_thinking.push_str(&text);
                            tokens_since_save += 1;
                            let _ = channel.send(StreamEvent::Thinking { content: text });
                        }
                        ParsedEvent::ToolCallDelta { index, id, name, arguments } => {
                            let _ = channel.send(StreamEvent::ToolCall {
                                index,
                                name: name.clone().unwrap_or_default(),
                                arguments: arguments.clone(),
                            });
                            if let Some(tc) = accum.iter_mut().find(|t| t.0 == index) {
                                if let Some(n) = &name {
                                    tc.1 = Some(n.clone());
                                }
                                if let Some(real_id) = &id {
                                    if !real_id.is_empty() {
                                        tc.2 = real_id.clone();
                                    }
                                }
                                tc.3.push_str(&arguments);
                            } else {
                                let entry_id = id
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or_else(|| format!("call_{}", uuid::Uuid::new_v4().simple()));
                                accum.push((index, name, entry_id, arguments));
                            }
                        }
                        ParsedEvent::Done => {
                            stream_done = true;
                            break;
                        }
                        ParsedEvent::Error(err) => {
                            let _ = channel.send(StreamEvent::Error { message: err });
                        }
                    }
                }
                // 节流落库, 防止取消时丢失大量已生成 token
                if tokens_since_save >= PERSIST_TOKEN_THRESHOLD
                    || last_save.elapsed() >= PERSIST_INTERVAL
                {
                    persist_partial(
                        db, msg_id, content_prefix, &full_content,
                        thinking_prefix, &full_thinking,
                    ).await;
                    tokens_since_save = 0;
                    last_save = Instant::now();
                }
            }
            Err(e) => {
                let _ = channel.send(StreamEvent::Error { message: e.to_string() });
                break;
            }
        }
    }

    // 退出前最后一次落库, 避免最后几个 token 丢失
    if tokens_since_save > 0 {
        persist_partial(
            db, msg_id, content_prefix, &full_content,
            thinking_prefix, &full_thinking,
        ).await;
    }

    let tool_calls: Vec<ToolCall> = accum.into_iter()
        .filter(|(_, name, _, _)| name.is_some())
        .map(|(_, name, id, args)| ToolCall {
            id,
            name: name.unwrap_or_default(),
            arguments: args,
        })
        .collect();

    RoundResult { content: full_content, thinking: full_thinking, tool_calls }
}

/// 加载会话历史(仅保留每个 parent_id 分组的最新版本)
async fn load_filtered_history(db: &SqlitePool, session_id: &str) -> Result<Vec<ChatMessage>, String> {
    let rows = sqlx::query(
        "SELECT * FROM (\
            SELECT *, ROW_NUMBER() OVER (\
                PARTITION BY COALESCE(parent_id, id), role \
                ORDER BY version DESC\
            ) as rn \
            FROM messages WHERE session_id = ?\
        ) WHERE rn = 1 ORDER BY created_at ASC"
    )
    .bind(session_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(|row| ChatMessage {
        id: row.get("id"),
        session_id: row.get("session_id"),
        role: row.get("role"),
        content: row.get("content"),
        version: row.get("version"),
        parent_id: row.get("parent_id"),
        created_at: row.get("created_at"),
        token_count: row.get("token_count"),
        error: row.get("error"),
        thinking: row.try_get("thinking").unwrap_or(None),
    }).collect())
}

/// 运行多轮 tool calling 循环并最终持久化
#[allow(clippy::too_many_arguments)]
async fn run_tool_loop(
    first_response: reqwest::Response,
    channel: Channel<StreamEvent>,
    msg_id: String,
    db: SqlitePool,
    session_id: String,
    model_name: String,
    provider_name: String,
    base_url: String,
    api_key: String,
    max_tokens: i32,
    temperature: f64,
    top_p: f64,
    temperature_enabled: bool,
    top_p_enabled: bool,
    reasoning_effort: Option<String>,
    passback_reasoning: bool,
    effective_prompt: Option<String>,
    request_body_str: String,
    status_code: i32,
    start_time: std::time::Instant,
    cancel_flag: Arc<AtomicBool>,
    ai_shell: crate::terminal::ai_shell::AiShellManager,
    app: tauri::AppHandle,
) {
    // 整个循环加 5 分钟硬超时, 超时则按取消处理
    let inner = run_tool_loop_inner(
        first_response, channel, msg_id, db, session_id,
        model_name, provider_name, base_url, api_key,
        max_tokens, temperature, top_p,
        temperature_enabled, top_p_enabled, reasoning_effort, passback_reasoning,
        effective_prompt, request_body_str, status_code, start_time,
        cancel_flag.clone(), ai_shell, app,
    );
    if tokio::time::timeout(Duration::from_secs(TOOL_LOOP_TIMEOUT_SECS), inner)
        .await
        .is_err()
    {
        cancel_flag.store(true, Ordering::Relaxed);
    }
}

/// 工具循环主体, 由外层 timeout 守护
#[allow(clippy::too_many_arguments)]
async fn run_tool_loop_inner(
    first_response: reqwest::Response,
    channel: Channel<StreamEvent>,
    msg_id: String,
    db: SqlitePool,
    session_id: String,
    model_name: String,
    provider_name: String,
    base_url: String,
    api_key: String,
    max_tokens: i32,
    temperature: f64,
    top_p: f64,
    temperature_enabled: bool,
    top_p_enabled: bool,
    reasoning_effort: Option<String>,
    passback_reasoning: bool,
    effective_prompt: Option<String>,
    request_body_str: String,
    status_code: i32,
    start_time: std::time::Instant,
    cancel_flag: Arc<AtomicBool>,
    ai_shell: crate::terminal::ai_shell::AiShellManager,
    app: tauri::AppHandle,
) {
    let registry = ToolRegistry::new();
    let tool_schemas = registry.get_schemas();

    // 累积每轮快照, 末尾随消息一起持久化
    let mut rounds: Vec<RoundSnapshot> = Vec::new();

    let first = consume_stream(
        first_response, &channel, &msg_id, &cancel_flag,
        &db, "", "",
    ).await;
    let mut full_content = first.content.clone();
    let mut full_thinking = first.thinking.clone();
    let mut accumulated_tool_calls = first.tool_calls.clone();

    // 推入首轮快照, round_end 延迟到工具执行完毕后发送
    rounds.push(RoundSnapshot {
        thinking: first.thinking.clone(),
        content: first.content.clone(),
        tool_calls: first.tool_calls.iter().map(|tc| RoundToolCallSnapshot {
            name: tc.name.clone(),
            arguments: tc.arguments.clone(),
        }).collect(),
        tool_execs: Vec::new(),
    });

    let mut iter_count = 0;
    const MAX_ITERATIONS: usize = 8;

    while !accumulated_tool_calls.is_empty()
        && !cancel_flag.load(Ordering::Relaxed)
        && iter_count < MAX_ITERATIONS
    {
        iter_count += 1;

        // 构造 assistant 的 tool_calls 消息和工具结果消息
        let tc_value: Vec<Value> = accumulated_tool_calls.iter().enumerate().map(|(i, tc)| {
            json!({
                "id": tc.id,
                "type": "function",
                "function": { "name": tc.name, "arguments": tc.arguments },
                "index": i,
            })
        }).collect();

        let mut tool_results_msgs: Vec<Value> = Vec::new();
        let mut assistant_msg = json!({
            "role": "assistant",
            "content": if full_content.is_empty() { Value::Null } else { Value::String(full_content.clone()) },
            "tool_calls": tc_value,
        });
        // 思考回传开关开启时, 把当前轮思考内容附加到 assistant 消息
        if passback_reasoning && !full_thinking.is_empty() {
            assistant_msg["reasoning_content"] = json!(full_thinking.clone());
        }
        tool_results_msgs.push(assistant_msg);

        // 执行每个工具
        for tc in &accumulated_tool_calls {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            // shell_exec 需要用户确认
            let needs_confirm = tc.name == "shell_exec";
            if needs_confirm {
                let _ = channel.send(StreamEvent::ToolConfirm {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    tool_call_id: tc.id.clone(),
                });
                let sm = app.state::<StreamManager>();
                let rx = sm.register_confirm(tc.id.clone()).await;
                // 等待用户确认, 同时监听取消
                let approved = tokio::select! {
                    r = rx => r.unwrap_or(false),
                    _ = wait_cancel(&cancel_flag) => false,
                };
                if !approved {
                    tool_results_msgs.push(json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": "用户拒绝执行此工具",
                    }));
                    if let Some(last) = rounds.last_mut() {
                        last.tool_execs.push(RoundToolExecSnapshot {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                            success: false,
                            result_preview: "用户拒绝执行".to_string(),
                        });
                    }
                    continue;
                }
            }

            let _ = channel.send(StreamEvent::ToolExecStart {
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            });
            let result = registry.execute(tc, &db, &ai_shell, &app, &session_id, &cancel_flag).await;
            let preview: String = result.content.chars().take(500).collect();
            let _ = channel.send(StreamEvent::ToolExecDone {
                name: tc.name.clone(),
                success: result.success,
                content: preview.clone(),
            });
            // 回填到当前最新轮快照
            if let Some(last) = rounds.last_mut() {
                last.tool_execs.push(RoundToolExecSnapshot {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    success: result.success,
                    result_preview: preview,
                });
            }
            tool_results_msgs.push(json!({
                "role": "tool",
                "tool_call_id": tc.id,
                "content": result.content,
            }));
        }

        // 工具执行完毕, 通知前端归档当前轮
        let _ = channel.send(StreamEvent::RoundEnd { round_index: rounds.len() - 1 });

        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        // 重新加载历史并追加新消息
        let history = load_filtered_history(&db, &session_id).await.unwrap_or_default();
        let mut messages_raw: Vec<Value> = Vec::new();
        if let Some(ref sp) = effective_prompt {
            if !sp.is_empty() {
                messages_raw.push(json!({"role": "system", "content": sp}));
            }
        }
        for m in &history {
            // 跳过当前正在生成的助手消息(内容为空)
            if m.id == msg_id { continue; }
            let mut item = json!({"role": m.role, "content": strip_rounds_marker(&m.content)});
            // 历史 assistant 消息按需附加思考内容
            if passback_reasoning && m.role == "assistant" {
                if let Some(t) = &m.thinking {
                    if !t.is_empty() {
                        item["reasoning_content"] = json!(t);
                    }
                }
            }
            messages_raw.push(item);
        }
        messages_raw.extend(tool_results_msgs);

        let body = LlmClient::build_request_body_raw(
            &model_name, &messages_raw, max_tokens, temperature, top_p,
            temperature_enabled, top_p_enabled, true, Some(&tool_schemas), reasoning_effort.as_deref(),
        );

        let client = LlmClient::new();
        // 与取消标记 race, 让请求阶段也能立即响应取消
        let send_fut = client.send_chat_stream(&base_url, &api_key, body);
        let send_result = tokio::select! {
            r = send_fut => Some(r),
            _ = wait_cancel(&cancel_flag) => None,
        };
        let send_result = match send_result {
            Some(r) => r,
            None => break,
        };
        match send_result {
            Ok(r) if r.status().is_success() => {
                // 后续轮次的内容追加, 用前缀传入以便节流落库
                let prefix_content = full_content.clone();
                let prefix_thinking = full_thinking.clone();
                let round = consume_stream(
                    r, &channel, &msg_id, &cancel_flag,
                    &db, &prefix_content, &prefix_thinking,
                ).await;
                if !round.content.is_empty() {
                    if !full_content.is_empty() {
                        full_content.push_str("\n\n");
                    }
                    full_content.push_str(&round.content);
                }
                if !round.thinking.is_empty() {
                    full_thinking.push_str(&round.thinking);
                }
                accumulated_tool_calls = round.tool_calls.clone();
                // 推入新一轮快照, round_end 延迟到下次工具执行完毕后发送
                rounds.push(RoundSnapshot {
                    thinking: round.thinking.clone(),
                    content: round.content.clone(),
                    tool_calls: round.tool_calls.iter().map(|tc| RoundToolCallSnapshot {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    }).collect(),
                    tool_execs: Vec::new(),
                });
            }
            Ok(r) => {
                let err_text = r.text().await.unwrap_or_default();
                let _ = channel.send(StreamEvent::Error { message: err_text });
                break;
            }
            Err(e) => {
                let _ = channel.send(StreamEvent::Error { message: e });
                break;
            }
        }
    }

    if cancel_flag.load(Ordering::Relaxed) {
        if full_content.is_empty() {
            full_content = "(已打断)".to_string();
        } else {
            full_content.push_str("\n\n(已打断)");
        }
        eprintln!("[stream] cancelled for session={}, msg={}", session_id, msg_id);
    } else {
        eprintln!("[stream] completed for session={}, msg={}, content_len={}", session_id, msg_id, full_content.len());
    }

    // 末尾附加分轮 marker, 取消路径同样需要保留已生成轮次
    let mut final_content = full_content.clone();
    if !rounds.is_empty() {
        if let Ok(rounds_json) = serde_json::to_string(&rounds) {
            if !final_content.is_empty() {
                final_content.push_str("\n\n");
            }
            final_content.push_str("<!-- chat_rounds:");
            final_content.push_str(&rounds_json);
            final_content.push_str(" -->");
        }
    }

    // 保存完整内容(必须在 Done 事件之前完成, 否则前端 loadMessages 拿不到内容)
    let _ = sqlx::query("UPDATE messages SET content = ?, thinking = ? WHERE id = ?")
        .bind(&final_content)
        .bind(if full_thinking.is_empty() { None } else { Some(&full_thinking) })
        .bind(&msg_id)
        .execute(&db)
        .await;

    if cancel_flag.load(Ordering::Relaxed) {
        let _ = channel.send(StreamEvent::Interrupted { message_id: msg_id.clone() });
        let _ = app.emit("stream-interrupted", serde_json::json!({
            "session_id": session_id,
            "message_id": msg_id,
        }));
    } else {
        let _ = channel.send(StreamEvent::Done { message_id: msg_id.clone() });
        let _ = app.emit("stream-finished", serde_json::json!({
            "session_id": session_id,
            "message_id": msg_id,
        }));
    }

    // 记录 API 日志
    let latency = start_time.elapsed().as_millis() as i64;
    let log_id = uuid::Uuid::new_v4().to_string();
    let log_now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO api_logs (id, session_id, model, provider, request_body, response_body, status_code, latency_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&log_id).bind(&session_id).bind(&model_name).bind(&provider_name)
    .bind(&request_body_str).bind(&full_content).bind(status_code).bind(latency).bind(&log_now)
    .execute(&db).await;
}

/// 发送消息并开始流式传输
#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    session_id: String,
    content: String,
    model_config_id: String,
    attachments: Vec<String>,
    default_system_prompt: Option<String>,
    channel: Channel<StreamEvent>,
    stream_manager: tauri::State<'_, StreamManager>,
    ai_shell_mgr: tauri::State<'_, crate::terminal::ai_shell::AiShellManager>,
) -> Result<String, String> {
    let db = pool.inner().clone();

    // 获取模型配置
    let config_row = sqlx::query("SELECT * FROM model_configs WHERE id = ?")
        .bind(&model_config_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("model config not found")?;

    let config = ModelConfig {
        id: config_row.get("id"),
        name: config_row.get("name"),
        provider: config_row.get("provider"),
        model: config_row.get("model"),
        base_url: config_row.get("base_url"),
        api_key_name: config_row.get("api_key_name"),
        api_key: config_row.try_get("api_key").unwrap_or(None),
        max_tokens: config_row.get("max_tokens"),
        context_window: config_row.try_get("context_window").unwrap_or(128000),
        temperature: config_row.get("temperature"),
        top_p: config_row.get("top_p"),
        temperature_enabled: config_row.try_get::<i32, _>("temperature_enabled").unwrap_or(1) == 1,
        top_p_enabled: config_row.try_get::<i32, _>("top_p_enabled").unwrap_or(1) == 1,
        system_prompt: config_row.get("system_prompt"),
        reasoning_effort: config_row.try_get("reasoning_effort").unwrap_or(None),
        passback_reasoning: config_row.try_get::<i32, _>("passback_reasoning").unwrap_or(0) == 1,
        retry_count: config_row.try_get("retry_count").unwrap_or(3),
        created_at: config_row.get("created_at"),
        updated_at: config_row.get("updated_at"),
    };

    let session_row = sqlx::query("SELECT system_prompt FROM sessions WHERE id = ?")
        .bind(&session_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| e.to_string())?;
    let session_prompt: Option<String> = session_row.and_then(|r| r.get("system_prompt"));

    let effective_prompt = session_prompt
        .or(config.system_prompt.clone())
        .or(default_system_prompt);

    let api_key = crate::commands::web_search::resolve_api_key(
        config.api_key_name.as_deref(),
        config.api_key.as_deref(),
    )?;

    // 保存用户消息(附件以 marker 形式追加到 content 末尾, 供前端渲染)
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let stored_content = if attachments.is_empty() {
        content.clone()
    } else {
        let marker = serde_json::to_string(&attachments).unwrap_or_default();
        format!("{}\n\n<!-- user_attachments:{} -->", content, marker)
    };
    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, 'user', ?, 1, NULL, ?, 0, NULL)"
    )
    .bind(&user_msg_id)
    .bind(&session_id)
    .bind(&stored_content)
    .bind(&now)
    .execute(&db)
    .await
    .map_err(|e| e.to_string())?;

    let messages = load_filtered_history(&db, &session_id).await?;

    let base_url = config.base_url.as_deref().unwrap_or("https://api.openai.com");
    let registry = ToolRegistry::new();
    let tool_schemas = registry.get_schemas();
    let mut body = LlmClient::build_request_body_full(
        &config.model,
        &messages,
        config.max_tokens,
        config.temperature,
        config.top_p,
        config.temperature_enabled,
        config.top_p_enabled,
        effective_prompt.as_deref(),
        true,
        Some(&tool_schemas),
        config.reasoning_effort.as_deref(),
        config.passback_reasoning,
    );

    // 将附件注入最后一条用户消息, 转为 multimodal content 格式
    if !attachments.is_empty() {
        if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
            if let Some(last_user) = msgs.iter_mut().rev().find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")) {
                let text = last_user.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                let mut parts: Vec<Value> = vec![json!({"type": "text", "text": text})];
                for data_uri in &attachments {
                    parts.push(json!({
                        "type": "image_url",
                        "image_url": {"url": data_uri}
                    }));
                }
                last_user["content"] = json!(parts);
            }
        }
    }

    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let stream_id = assistant_msg_id.clone();
    let assist_now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, 'assistant', '', 1, ?, ?, 0, NULL)"
    )
    .bind(&assistant_msg_id)
    .bind(&session_id)
    .bind(&user_msg_id)
    .bind(&assist_now)
    .execute(&db)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
        .bind(&assist_now)
        .bind(&session_id)
        .execute(&db)
        .await
        .map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let request_body_str = serde_json::to_string(&body).unwrap_or_default();

    let client = LlmClient::new();
    let retry_max = config.retry_count.max(1) as usize;
    let mut last_err;

    let mut response_opt: Option<reqwest::Response> = None;
    let mut final_status_code: i32 = 0;

    for attempt in 0..retry_max {
        let body_clone: Value = serde_json::from_str(&request_body_str).unwrap();
        match client.send_chat_stream(base_url, &api_key, body_clone).await {
            Ok(r) if r.status().is_success() => {
                final_status_code = r.status().as_u16() as i32;
                response_opt = Some(r);
                break;
            }
            Ok(r) => {
                final_status_code = r.status().as_u16() as i32;
                last_err = r.text().await.unwrap_or_default();
                if attempt + 1 >= retry_max {
                    let _ = channel.send(StreamEvent::Error { message: last_err.clone() });
                    let latency = start_time.elapsed().as_millis() as i64;
                    let log_id = uuid::Uuid::new_v4().to_string();
                    let log_now = chrono::Utc::now().to_rfc3339();
                    let _ = sqlx::query(
                        "INSERT INTO api_logs (id, session_id, model, provider, request_body, response_body, status_code, latency_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&log_id).bind(&session_id).bind(&config.model).bind(&config.provider)
                    .bind(&request_body_str).bind(&last_err).bind(final_status_code).bind(latency).bind(&log_now)
                    .execute(&db).await;
                    return Err(last_err);
                }
                tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
            Err(e) => {
                last_err = e;
                if attempt + 1 >= retry_max {
                    let _ = channel.send(StreamEvent::Error { message: last_err.clone() });
                    return Err(last_err);
                }
                tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
        }
    }

    let response = response_opt.unwrap();
    let status_code = final_status_code;

    let cancel_token = StreamManager::create_cancel_token();
    let cancel_flag = cancel_token.clone();
    let msg_id_clone = assistant_msg_id.clone();
    let db_clone = db.clone();
    let model_name = config.model.clone();
    let provider_name = config.provider.clone();
    let session_id_clone = session_id.clone();
    let base_url_str = base_url.to_string();
    let api_key_clone = api_key.clone();
    let max_tokens_v = config.max_tokens;
    let temperature_v = config.temperature;
    let top_p_v = config.top_p;
    let temperature_enabled_v = config.temperature_enabled;
    let top_p_enabled_v = config.top_p_enabled;
    let reasoning_effort_v = config.reasoning_effort.clone();
    let passback_reasoning_v = config.passback_reasoning;
    let effective_prompt_clone = effective_prompt.clone();
    let ai_shell = ai_shell_mgr.inner().clone();

    let handle = tokio::spawn(async move {
        run_tool_loop(
            response, channel, msg_id_clone, db_clone, session_id_clone,
            model_name, provider_name, base_url_str, api_key_clone,
            max_tokens_v, temperature_v, top_p_v,
            temperature_enabled_v, top_p_enabled_v, reasoning_effort_v, passback_reasoning_v,
            effective_prompt_clone, request_body_str, status_code, start_time,
            cancel_flag, ai_shell, app,
        ).await;
    });

    stream_manager.register(stream_id, handle, cancel_token).await;
    Ok(assistant_msg_id)
}

/// 取消正在进行的流式传输(用户打断)
#[tauri::command]
pub async fn cancel_stream(
    stream_id: String,
    stream_manager: tauri::State<'_, StreamManager>,
) -> Result<(), String> {
    stream_manager.cancel(&stream_id).await;
    Ok(())
}

/// 用户确认/拒绝工具执行
#[tauri::command]
pub async fn confirm_tool(
    tool_call_id: String,
    approved: bool,
    stream_manager: tauri::State<'_, StreamManager>,
) -> Result<(), String> {
    stream_manager.resolve_confirm(&tool_call_id, approved).await;
    Ok(())
}

/// 重新生成助手消息(创建新版本而非删除旧消息)
#[tauri::command]
pub async fn regenerate(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    message_id: String,
    model_config_id: String,
    default_system_prompt: Option<String>,
    channel: Channel<StreamEvent>,
    stream_manager: tauri::State<'_, StreamManager>,
    ai_shell_mgr: tauri::State<'_, crate::terminal::ai_shell::AiShellManager>,
) -> Result<String, String> {
    let db = pool.inner().clone();

    let old_row = sqlx::query("SELECT session_id, parent_id FROM messages WHERE id = ?")
        .bind(&message_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("message not found")?;

    let session_id: String = old_row.get("session_id");
    let parent_id: Option<String> = old_row.get("parent_id");
    let user_msg_id = parent_id.ok_or("original message has no parent_id")?;

    let max_ver_row = sqlx::query(
        "SELECT COALESCE(MAX(version), 0) as max_ver FROM messages WHERE parent_id = ?"
    )
    .bind(&user_msg_id)
    .fetch_one(&db)
    .await
    .map_err(|e| e.to_string())?;
    let new_version: i32 = max_ver_row.get::<i32, _>("max_ver") + 1;

    let config_row = sqlx::query("SELECT * FROM model_configs WHERE id = ?")
        .bind(&model_config_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("model config not found")?;

    let config = ModelConfig {
        id: config_row.get("id"),
        name: config_row.get("name"),
        provider: config_row.get("provider"),
        model: config_row.get("model"),
        base_url: config_row.get("base_url"),
        api_key_name: config_row.get("api_key_name"),
        api_key: config_row.try_get("api_key").unwrap_or(None),
        max_tokens: config_row.get("max_tokens"),
        context_window: config_row.try_get("context_window").unwrap_or(128000),
        temperature: config_row.get("temperature"),
        top_p: config_row.get("top_p"),
        temperature_enabled: config_row.try_get::<i32, _>("temperature_enabled").unwrap_or(1) == 1,
        top_p_enabled: config_row.try_get::<i32, _>("top_p_enabled").unwrap_or(1) == 1,
        system_prompt: config_row.get("system_prompt"),
        reasoning_effort: config_row.try_get("reasoning_effort").unwrap_or(None),
        passback_reasoning: config_row.try_get::<i32, _>("passback_reasoning").unwrap_or(0) == 1,
        retry_count: config_row.try_get("retry_count").unwrap_or(3),
        created_at: config_row.get("created_at"),
        updated_at: config_row.get("updated_at"),
    };

    let api_key = crate::commands::web_search::resolve_api_key(
        config.api_key_name.as_deref(),
        config.api_key.as_deref(),
    )?;

    let session_prompt_row = sqlx::query("SELECT system_prompt FROM sessions WHERE id = ?")
        .bind(&session_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| e.to_string())?;
    let session_prompt: Option<String> = session_prompt_row.and_then(|r| r.get("system_prompt"));

    let effective_prompt = session_prompt
        .or(config.system_prompt.clone())
        .or(default_system_prompt);

    let all_messages = load_filtered_history(&db, &session_id).await?;

    let user_row = sqlx::query("SELECT * FROM messages WHERE id = ?")
        .bind(&user_msg_id)
        .fetch_one(&db)
        .await
        .map_err(|e| e.to_string())?;
    let user_msg = ChatMessage {
        id: user_row.get("id"),
        session_id: user_row.get("session_id"),
        role: user_row.get("role"),
        content: user_row.get("content"),
        version: user_row.get("version"),
        parent_id: user_row.get("parent_id"),
        created_at: user_row.get("created_at"),
        token_count: user_row.get("token_count"),
        error: user_row.get("error"),
        thinking: user_row.try_get("thinking").unwrap_or(None),
    };

    let mut history: Vec<ChatMessage> = all_messages
        .into_iter()
        .take_while(|m| m.id != user_msg_id)
        .collect();
    history.push(user_msg);

    let base_url = config.base_url.as_deref().unwrap_or("https://api.openai.com");
    let registry = ToolRegistry::new();
    let tool_schemas = registry.get_schemas();
    let body = LlmClient::build_request_body_full(
        &config.model,
        &history,
        config.max_tokens,
        config.temperature,
        config.top_p,
        config.temperature_enabled,
        config.top_p_enabled,
        effective_prompt.as_deref(),
        true,
        Some(&tool_schemas),
        config.reasoning_effort.as_deref(),
        config.passback_reasoning,
    );

    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let stream_id = assistant_msg_id.clone();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, 'assistant', '', ?, ?, ?, 0, NULL)"
    )
    .bind(&assistant_msg_id)
    .bind(&session_id)
    .bind(new_version)
    .bind(&user_msg_id)
    .bind(&now)
    .execute(&db)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&session_id)
        .execute(&db)
        .await
        .map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let request_body_str = serde_json::to_string(&body).unwrap_or_default();

    let client = LlmClient::new();
    let retry_max = config.retry_count.max(1) as usize;
    let mut last_err;

    let mut response_opt: Option<reqwest::Response> = None;
    let mut final_status_code: i32 = 0;

    for attempt in 0..retry_max {
        let body_clone: Value = serde_json::from_str(&request_body_str).unwrap();
        match client.send_chat_stream(base_url, &api_key, body_clone).await {
            Ok(r) if r.status().is_success() => {
                final_status_code = r.status().as_u16() as i32;
                response_opt = Some(r);
                break;
            }
            Ok(r) => {
                final_status_code = r.status().as_u16() as i32;
                last_err = r.text().await.unwrap_or_default();
                if attempt + 1 >= retry_max {
                    let _ = channel.send(StreamEvent::Error { message: last_err.clone() });
                    let latency = start_time.elapsed().as_millis() as i64;
                    let log_id = uuid::Uuid::new_v4().to_string();
                    let log_now = chrono::Utc::now().to_rfc3339();
                    let _ = sqlx::query(
                        "INSERT INTO api_logs (id, session_id, model, provider, request_body, response_body, status_code, latency_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&log_id).bind(&session_id).bind(&config.model).bind(&config.provider)
                    .bind(&request_body_str).bind(&last_err).bind(final_status_code).bind(latency).bind(&log_now)
                    .execute(&db).await;
                    return Err(last_err);
                }
                tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
            Err(e) => {
                last_err = e;
                if attempt + 1 >= retry_max {
                    let _ = channel.send(StreamEvent::Error { message: last_err.clone() });
                    return Err(last_err);
                }
                tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
        }
    }

    let response = response_opt.unwrap();
    let status_code = final_status_code;

    let cancel_token = StreamManager::create_cancel_token();
    let cancel_flag = cancel_token.clone();
    let msg_id_clone = assistant_msg_id.clone();
    let db_clone = db.clone();
    let model_name = config.model.clone();
    let provider_name = config.provider.clone();
    let session_id_clone = session_id.clone();
    let base_url_str = base_url.to_string();
    let api_key_clone = api_key.clone();
    let max_tokens_v = config.max_tokens;
    let temperature_v = config.temperature;
    let top_p_v = config.top_p;
    let temperature_enabled_v = config.temperature_enabled;
    let top_p_enabled_v = config.top_p_enabled;
    let reasoning_effort_v = config.reasoning_effort.clone();
    let passback_reasoning_v = config.passback_reasoning;
    let effective_prompt_clone = effective_prompt.clone();
    let ai_shell = ai_shell_mgr.inner().clone();

    let handle = tokio::spawn(async move {
        run_tool_loop(
            response, channel, msg_id_clone, db_clone, session_id_clone,
            model_name, provider_name, base_url_str, api_key_clone,
            max_tokens_v, temperature_v, top_p_v,
            temperature_enabled_v, top_p_enabled_v, reasoning_effort_v, passback_reasoning_v,
            effective_prompt_clone, request_body_str, status_code, start_time,
            cancel_flag, ai_shell, app,
        ).await;
    });

    stream_manager.register(stream_id, handle, cancel_token).await;
    Ok(assistant_msg_id)
}
