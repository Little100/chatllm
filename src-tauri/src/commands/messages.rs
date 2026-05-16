use crate::models::ChatMessage;
use sqlx::{Row, SqlitePool};

/// 当前未注册到 IPC, 保留供导入流程使用
#[allow(dead_code)]
#[tauri::command]
pub async fn create_message(
    pool: tauri::State<'_, SqlitePool>,
    session_id: String,
    role: String,
    content: String,
) -> Result<ChatMessage, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, ?, ?, 1, NULL, ?, 0, NULL)"
    )
    .bind(&id)
    .bind(&session_id)
    .bind(&role)
    .bind(&content)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&session_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ChatMessage {
        id,
        session_id,
        role,
        content,
        version: 1,
        parent_id: None,
        created_at: now,
        token_count: 0,
        error: None,
        thinking: None,
    })
}

/// 列出会话消息(用户消息全保留, 助手消息按 parent_id 仅保留最新版本)
#[tauri::command]
pub async fn list_messages(
    pool: tauri::State<'_, SqlitePool>,
    session_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let rows = sqlx::query(
        "SELECT * FROM (\
            SELECT *, ROW_NUMBER() OVER (\
                PARTITION BY \
                    CASE WHEN parent_id IS NULL THEN id ELSE parent_id END, \
                    role \
                ORDER BY version DESC, created_at DESC\
            ) as rn \
            FROM messages WHERE session_id = ?\
        ) WHERE rn = 1 ORDER BY created_at ASC"
    )
    .bind(&session_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let messages = rows
        .iter()
        .map(|row| ChatMessage {
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
        })
        .collect();
    Ok(messages)
}

/// 删除消息
#[tauri::command]
pub async fn delete_message(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 编辑消息内容
#[tauri::command]
pub async fn update_message(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
    content: String,
) -> Result<(), String> {
    let db = pool.inner();
    // 先更新消息, 再用同一连接刷新归属会话的 updated_at
    sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
        .bind(&content)
        .bind(&id)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sessions SET updated_at = ? WHERE id = (SELECT session_id FROM messages WHERE id = ?)"
    )
    .bind(&now)
    .bind(&id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 切换到指定 parent_id 下的目标版本
#[tauri::command]
pub async fn switch_version(
    pool: tauri::State<'_, SqlitePool>,
    parent_id: String,
    version: i32,
) -> Result<ChatMessage, String> {
    let row = sqlx::query(
        "SELECT * FROM messages WHERE parent_id = ? AND version = ?"
    )
    .bind(&parent_id)
    .bind(version)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "version not found".to_string())?;

    Ok(ChatMessage {
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
    })
}

/// 获取指定 parent_id 下的所有版本信息
#[tauri::command]
pub async fn get_message_versions(
    pool: tauri::State<'_, SqlitePool>,
    parent_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let rows = sqlx::query(
        "SELECT * FROM messages WHERE parent_id = ? ORDER BY version ASC"
    )
    .bind(&parent_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let messages = rows
        .iter()
        .map(|row| ChatMessage {
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
        })
        .collect();
    Ok(messages)
}
