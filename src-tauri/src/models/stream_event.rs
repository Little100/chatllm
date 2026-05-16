use serde::Serialize;

/// 流式事件枚举，用于 Tauri Channel
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// 收到一个 token
    #[serde(rename = "token")]
    Token { content: String },
    /// 思维链增量
    #[serde(rename = "thinking")]
    Thinking { content: String },
    /// 工具调用增量
    #[serde(rename = "tool_call")]
    ToolCall { index: usize, name: String, arguments: String },
    /// 工具等待用户确认
    #[serde(rename = "tool_confirm")]
    ToolConfirm { name: String, arguments: String, tool_call_id: String },
    /// 工具开始执行
    #[serde(rename = "tool_exec_start")]
    ToolExecStart { name: String, arguments: String },
    /// 工具执行完成
    #[serde(rename = "tool_exec_done")]
    ToolExecDone { name: String, success: bool, content: String },
    /// 流式传输完成
    #[serde(rename = "done")]
    Done { message_id: String },
    /// 用户打断
    #[serde(rename = "interrupted")]
    Interrupted { message_id: String },
    /// 传输错误
    #[serde(rename = "error")]
    Error { message: String },
    /// 当前轮次 LLM 响应结束(尚未进入工具执行或全流程结束)
    #[serde(rename = "round_end")]
    RoundEnd { round_index: usize },
}
