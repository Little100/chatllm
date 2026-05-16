use serde::{Deserialize, Serialize};

/// 搜索配置结构体(独立模型设置)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub id: String,
    pub name: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key_name: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: i32,
    pub context_window: i32,
    pub temperature: f64,
    pub top_p: f64,
    pub temperature_enabled: bool,
    pub top_p_enabled: bool,
    pub reasoning_effort: Option<String>,
    pub prompt_template: String,
    pub enabled: bool,
    pub created_at: String,
}
