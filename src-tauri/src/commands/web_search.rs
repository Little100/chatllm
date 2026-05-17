use crate::models::SearchConfig;
use crate::services::llm_client::LlmClient;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Row, SqlitePool};

/// 创建搜索配置参数(独立模型设置)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSearchConfigParams {
    pub name: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key_name: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: i32,
    #[serde(default = "default_context_window")]
    pub context_window: i32,
    pub temperature: f64,
    pub top_p: f64,
    #[serde(default = "default_true")]
    pub temperature_enabled: bool,
    #[serde(default = "default_true")]
    pub top_p_enabled: bool,
    pub reasoning_effort: Option<String>,
    pub prompt_template: String,
    pub enabled: bool,
}

fn default_context_window() -> i32 { 128000 }
fn default_true() -> bool { true }

/// 写入 keyring(最佳努力, 失败仅记录日志)
fn try_persist_keyring(service: &str, key: &str) {
    match keyring::Entry::new("chatllm", service) {
        Ok(entry) => {
            if let Err(e) = entry.set_password(key) {
                eprintln!("(web_search) keyring set_password 失败 service={} err={}", service, e);
            }
        }
        Err(e) => {
            eprintln!("(web_search) keyring 打开失败 service={} err={}", service, e);
        }
    }
}

/// 解析存储的 API Key, 先查 keyring 再回退到 DB 列
pub fn resolve_api_key(api_key_name: Option<&str>, api_key: Option<&str>) -> Result<String, String> {
    if let Some(key_name) = api_key_name {
        if let Ok(entry) = keyring::Entry::new("chatllm", key_name) {
            if let Ok(pw) = entry.get_password() {
                if !pw.is_empty() {
                    return Ok(pw);
                }
            }
        }
    }
    if let Some(k) = api_key {
        if !k.is_empty() {
            return Ok(k.to_string());
        }
    }
    Err("未找到 API Key, 请在模型配置中重新填写".to_string())
}

/// 当前未注册到 IPC, 保留供未来按需启用
#[allow(dead_code)]
#[tauri::command]
pub async fn search(
    pool: tauri::State<'_, SqlitePool>,
    query: String,
) -> Result<String, String> {
    let row = sqlx::query("SELECT * FROM search_configs WHERE enabled = 1 LIMIT 1")
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "未配置搜索模型，请在设置中添加搜索配置".to_string())?;

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
    let api_key = resolve_api_key(api_key_name.as_deref(), api_key_db.as_deref())?;

    let prompt = prompt_template.replace("{query}", &query);

    // 构建非流式请求
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

    let client = LlmClient::new();
    let resp = client.send_chat(&base_url, &api_key, body).await?;

    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if content.is_empty() {
        return Err("搜索模型未返回有效内容".to_string());
    }
    Ok(content)
}

/// 行转结构体
fn row_to_config(row: &sqlx::sqlite::SqliteRow) -> SearchConfig {
    let enabled_int: i32 = row.get("enabled");
    let temp_enabled: i32 = row.try_get("temperature_enabled").unwrap_or(1);
    let top_p_enabled: i32 = row.try_get("top_p_enabled").unwrap_or(1);
    SearchConfig {
        id: row.get("id"),
        name: row.get("name"),
        model: row.get("model"),
        base_url: row.get("base_url"),
        api_key_name: row.get("api_key_name"),
        api_key: row.try_get("api_key").unwrap_or(None),
        max_tokens: row.get("max_tokens"),
        context_window: row.try_get("context_window").unwrap_or(128000),
        temperature: row.get("temperature"),
        top_p: row.get("top_p"),
        temperature_enabled: temp_enabled == 1,
        top_p_enabled: top_p_enabled == 1,
        reasoning_effort: row.try_get("reasoning_effort").unwrap_or(None),
        prompt_template: row.get("prompt_template"),
        enabled: enabled_int == 1,
        created_at: row.get("created_at"),
    }
}

/// 创建搜索配置
#[tauri::command]
pub async fn create_search_config(
    pool: tauri::State<'_, SqlitePool>,
    params: CreateSearchConfigParams,
) -> Result<SearchConfig, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let enabled_int: i32 = if params.enabled { 1 } else { 0 };

    if let (Some(name), Some(key)) = (params.api_key_name.as_deref(), params.api_key.as_deref()) {
        if !key.is_empty() {
            try_persist_keyring(name, key);
        }
    }

    sqlx::query(
        "INSERT INTO search_configs (id, name, model, base_url, api_key_name, api_key, max_tokens, context_window, temperature, top_p, temperature_enabled, top_p_enabled, reasoning_effort, prompt_template, enabled, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&params.name)
    .bind(&params.model)
    .bind(&params.base_url)
    .bind(&params.api_key_name)
    .bind(&params.api_key)
    .bind(params.max_tokens)
    .bind(params.context_window)
    .bind(params.temperature)
    .bind(params.top_p)
    .bind(params.temperature_enabled as i32)
    .bind(params.top_p_enabled as i32)
    .bind(&params.reasoning_effort)
    .bind(&params.prompt_template)
    .bind(enabled_int)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(SearchConfig {
        id,
        name: params.name,
        model: params.model,
        base_url: params.base_url,
        api_key_name: params.api_key_name,
        api_key: params.api_key,
        max_tokens: params.max_tokens,
        context_window: params.context_window,
        temperature: params.temperature,
        top_p: params.top_p,
        temperature_enabled: params.temperature_enabled,
        top_p_enabled: params.top_p_enabled,
        reasoning_effort: params.reasoning_effort,
        prompt_template: params.prompt_template,
        enabled: params.enabled,
        created_at: now,
    })
}

/// 列出所有搜索配置
#[tauri::command]
pub async fn list_search_configs(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<SearchConfig>, String> {
    let rows = sqlx::query("SELECT * FROM search_configs ORDER BY created_at ASC")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_config).collect())
}

/// 更新搜索配置
#[tauri::command]
pub async fn update_search_config(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
    params: CreateSearchConfigParams,
) -> Result<(), String> {
    let enabled_int: i32 = if params.enabled { 1 } else { 0 };

    if let (Some(name), Some(key)) = (params.api_key_name.as_deref(), params.api_key.as_deref()) {
        if !key.is_empty() {
            try_persist_keyring(name, key);
        }
    }

    sqlx::query(
        "UPDATE search_configs SET name=?, model=?, base_url=?, api_key_name=?, api_key=?, max_tokens=?, context_window=?, temperature=?, top_p=?, temperature_enabled=?, top_p_enabled=?, reasoning_effort=?, prompt_template=?, enabled=? WHERE id=?"
    )
    .bind(&params.name)
    .bind(&params.model)
    .bind(&params.base_url)
    .bind(&params.api_key_name)
    .bind(&params.api_key)
    .bind(params.max_tokens)
    .bind(params.context_window)
    .bind(params.temperature)
    .bind(params.top_p)
    .bind(params.temperature_enabled as i32)
    .bind(params.top_p_enabled as i32)
    .bind(&params.reasoning_effort)
    .bind(&params.prompt_template)
    .bind(enabled_int)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除搜索配置
#[tauri::command]
pub async fn delete_search_config(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM search_configs WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
