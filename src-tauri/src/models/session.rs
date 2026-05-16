use serde::{Deserialize, Serialize};

/// 会话结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub model_config_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub pinned: bool,
    pub archived: bool,
    pub system_prompt: Option<String>,
}
