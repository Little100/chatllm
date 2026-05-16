use crate::models::ModelConfig;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Row, SqlitePool};

/// 创建模型配置参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelConfigParams {
    pub name: String,
    pub provider: String,
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
    pub system_prompt: Option<String>,
    pub reasoning_effort: Option<String>,
    pub passback_reasoning: Option<bool>,
    #[serde(default = "default_retry_count")]
    pub retry_count: i32,
}

fn default_context_window() -> i32 { 128000 }
fn default_true() -> bool { true }
fn default_retry_count() -> i32 { 3 }

/// 尝试同步保存到系统密钥库(失败不阻塞)
fn try_persist_keyring(service: &str, key: &str) {
    if let Ok(entry) = keyring::Entry::new("chatllm", service) {
        let _ = entry.set_password(key);
    }
}

/// 创建模型配置
#[tauri::command]
pub async fn create_model_config(
    pool: tauri::State<'_, SqlitePool>,
    params: CreateModelConfigParams,
) -> Result<ModelConfig, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    if let (Some(name), Some(key)) = (params.api_key_name.as_deref(), params.api_key.as_deref()) {
        try_persist_keyring(name, key);
    }

    sqlx::query(
        "INSERT INTO model_configs (id, name, provider, model, base_url, api_key_name, api_key, max_tokens, context_window, temperature, top_p, temperature_enabled, top_p_enabled, system_prompt, reasoning_effort, passback_reasoning, retry_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&params.name)
    .bind(&params.provider)
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
    .bind(&params.system_prompt)
    .bind(&params.reasoning_effort)
    .bind(params.passback_reasoning.unwrap_or(false) as i32)
    .bind(params.retry_count)
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(ModelConfig {
        id,
        name: params.name,
        provider: params.provider,
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
        system_prompt: params.system_prompt,
        reasoning_effort: params.reasoning_effort,
        passback_reasoning: params.passback_reasoning.unwrap_or(false),
        retry_count: params.retry_count,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 列出所有模型配置
#[tauri::command]
pub async fn list_model_configs(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<ModelConfig>, String> {
    let rows = sqlx::query("SELECT * FROM model_configs ORDER BY created_at ASC")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let configs = rows
        .iter()
        .map(|row| {
            let temp_enabled: i32 = row.try_get("temperature_enabled").unwrap_or(1);
            let top_p_enabled: i32 = row.try_get("top_p_enabled").unwrap_or(1);
            ModelConfig {
                id: row.get("id"),
                name: row.get("name"),
                provider: row.get("provider"),
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
                system_prompt: row.get("system_prompt"),
                reasoning_effort: row.try_get("reasoning_effort").unwrap_or(None),
                passback_reasoning: row.try_get::<i32, _>("passback_reasoning").unwrap_or(0) == 1,
                retry_count: row.try_get("retry_count").unwrap_or(3),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
        .collect();
    Ok(configs)
}

/// 更新模型配置
#[tauri::command]
pub async fn update_model_config(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
    params: CreateModelConfigParams,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    if let (Some(name), Some(key)) = (params.api_key_name.as_deref(), params.api_key.as_deref()) {
        try_persist_keyring(name, key);
    }

    sqlx::query(
        "UPDATE model_configs SET name=?, provider=?, model=?, base_url=?, api_key_name=?, api_key=?, max_tokens=?, context_window=?, temperature=?, top_p=?, temperature_enabled=?, top_p_enabled=?, system_prompt=?, reasoning_effort=?, passback_reasoning=?, retry_count=?, updated_at=? WHERE id=?"
    )
    .bind(&params.name)
    .bind(&params.provider)
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
    .bind(&params.system_prompt)
    .bind(&params.reasoning_effort)
    .bind(params.passback_reasoning.unwrap_or(false) as i32)
    .bind(params.retry_count)
    .bind(&now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除模型配置
#[tauri::command]
pub async fn delete_model_config(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM model_configs WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 校验 base_url 必须为 https 或 http://localhost / 127.0.0.1
fn validate_base_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_lowercase();
    if lower.starts_with("https://") {
        return Ok(());
    }
    if lower.starts_with("http://") {
        let rest = &lower["http://".len()..];
        let host = rest.split(['/', ':']).next().unwrap_or("");
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            return Ok(());
        }
        return Err("非本地地址不允许使用 http, 请使用 https".to_string());
    }
    Err("base_url 必须以 https:// 或 http://localhost 开头".to_string())
}

/// 拉取远端模型列表
#[tauri::command]
pub async fn fetch_model_list(
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    validate_base_url(&base_url)?;
    let trimmed = base_url.trim_end_matches('/');
    let url = if trimmed.ends_with("/v1") || trimmed.ends_with("/v1/models") {
        if trimmed.ends_with("/models") {
            trimmed.to_string()
        } else {
            format!("{}/models", trimmed)
        }
    } else {
        format!("{}/v1/models", trimmed)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let data = json["data"].as_array().ok_or("响应缺少 data 字段")?;

    let mut ids: Vec<String> = data
        .iter()
        .filter_map(|item| item["id"].as_str().map(|s| s.to_string()))
        .collect();
    ids.sort();
    Ok(ids)
}
