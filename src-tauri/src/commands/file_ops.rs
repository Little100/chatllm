use base64::{engine::general_purpose::STANDARD, Engine};
use tauri::Manager;

/// 读取文件并返回 base64 编码
#[tauri::command]
pub async fn read_file_base64(path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("read failed: {}", e))?;
    Ok(STANDARD.encode(&bytes))
}

/// 当前未注册到 IPC, 保留供未来加 allowlist 后启用
#[allow(dead_code)]
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(&path, content.as_bytes())
        .await
        .map_err(|e| format!("write failed: {}", e))
}

/// 保存头像到应用配置目录
#[tauri::command]
pub async fn save_avatar(
    app: tauri::AppHandle,
    avatar_type: String,
    data: String,
) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let avatars_dir = config_dir.join("avatars");
    tokio::fs::create_dir_all(&avatars_dir)
        .await
        .map_err(|e| e.to_string())?;

    // 解码 base64 数据
    let bytes = STANDARD.decode(&data).map_err(|e| e.to_string())?;

    let filename = match avatar_type.as_str() {
        "user" => "user.png",
        "llm" => "llm.png",
        _ => return Err("invalid avatar type".to_string()),
    };

    let path = avatars_dir.join(filename);
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取头像 base64 数据
#[tauri::command]
pub async fn get_avatar(
    app: tauri::AppHandle,
    avatar_type: String,
) -> Result<Option<String>, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let filename = match avatar_type.as_str() {
        "user" => "user.png",
        "llm" => "llm.png",
        _ => return Err("invalid avatar type".to_string()),
    };

    let path = config_dir.join("avatars").join(filename);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(STANDARD.encode(&bytes)))
}

/// 删除头像文件
#[tauri::command]
pub async fn delete_avatar(
    app: tauri::AppHandle,
    avatar_type: String,
) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let filename = match avatar_type.as_str() {
        "user" => "user.png",
        "llm" => "llm.png",
        _ => return Err("invalid avatar type".to_string()),
    };
    let path = config_dir.join("avatars").join(filename);
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
