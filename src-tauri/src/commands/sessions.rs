use crate::models::Session;
use sqlx::{Row, SqlitePool};

/// 创建新会话
#[tauri::command]
pub async fn create_session(
    pool: tauri::State<'_, SqlitePool>,
    title: String,
    model_config_id: Option<String>,
) -> Result<Session, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO sessions (id, title, model_config_id, created_at, updated_at, pinned, archived) VALUES (?, ?, ?, ?, ?, 0, 0)"
    )
    .bind(&id)
    .bind(&title)
    .bind(&model_config_id)
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(Session {
        id,
        title,
        model_config_id,
        created_at: now.clone(),
        updated_at: now,
        pinned: false,
        archived: false,
        system_prompt: None,
    })
}

/// 列出所有会话
#[tauri::command]
pub async fn list_sessions(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Session>, String> {
    let rows = sqlx::query("SELECT * FROM sessions ORDER BY pinned DESC, updated_at DESC")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let sessions = rows
        .iter()
        .map(|row| Session {
            id: row.get("id"),
            title: row.get("title"),
            model_config_id: row.get("model_config_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            pinned: row.get::<i32, _>("pinned") != 0,
            archived: row.get::<i32, _>("archived") != 0,
            system_prompt: row.get("system_prompt"),
        })
        .collect();
    Ok(sessions)
}

/// 重命名会话
#[tauri::command]
pub async fn rename_session(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
    title: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&title)
        .bind(&now)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除会话(消息表 ON DELETE CASCADE 自动级联)
#[tauri::command]
pub async fn delete_session(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 当前未注册到 IPC, 保留供未来按需启用
#[allow(dead_code)]
#[tauri::command]
pub async fn search_sessions(
    pool: tauri::State<'_, SqlitePool>,
    query: String,
) -> Result<Vec<Session>, String> {
    let pattern = format!("%{}%", query);
    let rows =
        sqlx::query("SELECT * FROM sessions WHERE title LIKE ? ORDER BY updated_at DESC")
            .bind(&pattern)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    let sessions = rows
        .iter()
        .map(|row| Session {
            id: row.get("id"),
            title: row.get("title"),
            model_config_id: row.get("model_config_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            pinned: row.get::<i32, _>("pinned") != 0,
            archived: row.get::<i32, _>("archived") != 0,
            system_prompt: row.get("system_prompt"),
        })
        .collect();
    Ok(sessions)
}

/// 当前未注册到 IPC, 保留供未来按需启用
#[allow(dead_code)]
#[tauri::command]
pub async fn update_session_system_prompt(
    pool: tauri::State<'_, SqlitePool>,
    id: String,
    system_prompt: Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE sessions SET system_prompt = ?, updated_at = ? WHERE id = ?")
        .bind(&system_prompt)
        .bind(&now)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
