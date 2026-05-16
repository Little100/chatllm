use crate::terminal::pty_manager::PtyManager;

/// 创建终端会话
#[tauri::command]
pub async fn create_terminal(
    id: String,
    shell: Option<String>,
    app: tauri::AppHandle,
    pty_manager: tauri::State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.create_session(id, shell, app)
}

/// 写入终端数据
#[tauri::command]
pub async fn write_to_terminal(
    id: String,
    data: String,
    pty_manager: tauri::State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.write(&id, &data)
}

/// 调整终端尺寸
#[tauri::command]
pub async fn resize_terminal(
    id: String,
    cols: u16,
    rows: u16,
    pty_manager: tauri::State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.resize(&id, cols, rows)
}

/// 关闭终端
#[tauri::command]
pub async fn close_terminal(
    id: String,
    pty_manager: tauri::State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.close(&id)
}
