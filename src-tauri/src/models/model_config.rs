use serde::{Deserialize, Serialize};

/// 模型配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
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
    pub system_prompt: Option<String>,
    pub reasoning_effort: Option<String>,
    pub passback_reasoning: bool,
    pub retry_count: i32,
    pub created_at: String,
    pub updated_at: String,
}
