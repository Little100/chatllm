use sqlx::{Row, SqlitePool};

/// 导出会话为 JSON
#[tauri::command]
pub async fn export_json(
    pool: tauri::State<'_, SqlitePool>,
    session_id: String,
) -> Result<String, String> {
    let session_row = sqlx::query("SELECT * FROM sessions WHERE id = ?")
        .bind(&session_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let session_json = session_row.map(|row| {
        serde_json::json!({
            "id": row.get::<String, _>("id"),
            "title": row.get::<String, _>("title"),
            "model_config_id": row.get::<Option<String>, _>("model_config_id"),
            "created_at": row.get::<String, _>("created_at"),
            "updated_at": row.get::<String, _>("updated_at"),
        })
    });

    let msg_rows =
        sqlx::query("SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC")
            .bind(&session_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    let messages: Vec<serde_json::Value> = msg_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "role": row.get::<String, _>("role"),
                "content": row.get::<String, _>("content"),
                "version": row.get::<i32, _>("version"),
                "created_at": row.get::<String, _>("created_at"),
            })
        })
        .collect();

    let export = serde_json::json!({
        "session": session_json,
        "messages": messages,
    });

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// 导出会话为 Markdown
#[tauri::command]
pub async fn export_markdown(
    pool: tauri::State<'_, SqlitePool>,
    session_id: String,
) -> Result<String, String> {
    let title_row = sqlx::query("SELECT title FROM sessions WHERE id = ?")
        .bind(&session_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let title = title_row
        .map(|r| r.get::<String, _>("title"))
        .unwrap_or_else(|| "Untitled".to_string());

    let msg_rows = sqlx::query(
        "SELECT role, content FROM messages WHERE session_id = ? ORDER BY created_at ASC",
    )
    .bind(&session_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut md = format!("# {}\n\n", title);
    for row in &msg_rows {
        let role: String = row.get("role");
        let content: String = row.get("content");
        let label = match role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            _ => &role,
        };
        md.push_str(&format!("## {}\n\n{}\n\n---\n\n", label, content));
    }
    Ok(md)
}

/// 导入 JSON 会话
#[tauri::command]
pub async fn import_json(
    pool: tauri::State<'_, SqlitePool>,
    json_str: String,
) -> Result<String, String> {
    let data: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let title = data
        .get("session")
        .and_then(|s| s.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Chat");

    sqlx::query(
        "INSERT INTO sessions (id, title, model_config_id, created_at, updated_at, pinned, archived) VALUES (?, ?, NULL, ?, ?, 0, 0)"
    )
    .bind(&session_id)
    .bind(title)
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if let Some(messages) = data.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            let msg_id = uuid::Uuid::new_v4().to_string();
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let created_at = msg
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or(&now);

            sqlx::query(
                "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, ?, ?, 1, NULL, ?, 0, NULL)"
            )
            .bind(&msg_id)
            .bind(&session_id)
            .bind(role)
            .bind(content)
            .bind(created_at)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(session_id)
}

/// 导入 Markdown 会话
#[tauri::command]
pub async fn import_markdown(
    pool: tauri::State<'_, SqlitePool>,
    markdown: String,
) -> Result<String, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let title = markdown
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim())
        .unwrap_or("Imported Chat");

    sqlx::query(
        "INSERT INTO sessions (id, title, model_config_id, created_at, updated_at, pinned, archived) VALUES (?, ?, NULL, ?, ?, 0, 0)"
    )
    .bind(&session_id)
    .bind(title)
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let sections: Vec<&str> = markdown.split("\n## ").collect();
    for section in sections.iter().skip(1) {
        let mut lines = section.lines();
        let header = lines.next().unwrap_or("");
        let role = if header.contains("User") {
            "user"
        } else if header.contains("Assistant") {
            "assistant"
        } else {
            "system"
        };
        let content: String = lines
            .take_while(|l| *l != "---")
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        if !content.is_empty() {
            let msg_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO messages (id, session_id, role, content, version, parent_id, created_at, token_count, error) VALUES (?, ?, ?, ?, 1, NULL, ?, 0, NULL)"
            )
            .bind(&msg_id)
            .bind(&session_id)
            .bind(role)
            .bind(&content)
            .bind(&now)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(session_id)
}
