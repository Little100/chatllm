use crate::models::ApiLog;
use sqlx::{Row, SqlitePool};

/// 列出 API 日志
#[tauri::command]
pub async fn list_logs(
    pool: tauri::State<'_, SqlitePool>,
    limit: Option<i32>,
    offset: Option<i32>,
    model_filter: Option<String>,
) -> Result<Vec<ApiLog>, String> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    let rows = if let Some(model) = model_filter {
        sqlx::query(
            "SELECT * FROM api_logs WHERE model = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(&model)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.inner())
        .await
    } else {
        sqlx::query("SELECT * FROM api_logs ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(limit)
            .bind(offset)
            .fetch_all(pool.inner())
            .await
    }
    .map_err(|e| e.to_string())?;

    let logs = rows
        .iter()
        .map(|row| ApiLog {
            id: row.get("id"),
            session_id: row.get("session_id"),
            model: row.get("model"),
            provider: row.get("provider"),
            request_body: row.get("request_body"),
            response_body: row.get("response_body"),
            status_code: row.get("status_code"),
            latency_ms: row.get("latency_ms"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(logs)
}

/// 获取单条日志详情
#[tauri::command]
pub async fn get_log_detail(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
) -> Result<ApiLog, String> {
    let row = sqlx::query("SELECT * FROM api_logs WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "log not found".to_string())?;

    Ok(ApiLog {
        id: row.get("id"),
        session_id: row.get("session_id"),
        model: row.get("model"),
        provider: row.get("provider"),
        request_body: row.get("request_body"),
        response_body: row.get("response_body"),
        status_code: row.get("status_code"),
        latency_ms: row.get("latency_ms"),
        created_at: row.get("created_at"),
    })
}
