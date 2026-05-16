/// 保存 API Key 到系统密钥环
#[tauri::command]
pub async fn save_api_key(service: String, key: String) -> Result<(), String> {
    let entry = keyring::Entry::new("chatllm", &service).map_err(|e| e.to_string())?;
    entry.set_password(&key).map_err(|e| e.to_string())
}

/// 从系统密钥环获取 API Key
#[tauri::command]
pub async fn get_api_key(service: String) -> Result<String, String> {
    let entry = keyring::Entry::new("chatllm", &service).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

/// 从系统密钥环删除 API Key
#[tauri::command]
pub async fn delete_api_key(service: String) -> Result<(), String> {
    let entry = keyring::Entry::new("chatllm", &service).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
