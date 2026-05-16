use serde::{Deserialize, Serialize};

/// 聊天消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub version: i32,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub token_count: i32,
    pub error: Option<String>,
    pub thinking: Option<String>,
}
