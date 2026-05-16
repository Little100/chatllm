use serde::{Deserialize, Serialize};

/// API 日志结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiLog {
    pub id: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub status_code: Option<i32>,
    pub latency_ms: Option<i64>,
    pub created_at: String,
}
