use serde_json::Value;

/// 解析后的 SSE 事件
#[derive(Debug, Clone)]
pub enum ParsedEvent {
    /// 文本增量
    Token(String),
    /// 思维链增量
    Thinking(String),
    /// 工具调用增量(id 仅在首个 chunk 出现)
    ToolCallDelta { index: usize, id: Option<String>, name: Option<String>, arguments: String },
    /// 流结束
    Done,
    /// 错误信息
    Error(String),
}

/// 增量式 SSE 解析器
pub struct SseParser {
    buffer: Vec<u8>,
}

impl SseParser {
    /// 创建解析器
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    /// 投喂字节并返回解析事件
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ParsedEvent> {
        // 字节级累积避免多字节字符被切碎
        self.buffer.extend_from_slice(bytes);

        let mut events = Vec::new();
        let mut consumed = 0usize;

        // 按字节查找事件分隔符
        while let Some(pos) = find_double_newline(&self.buffer[consumed..]) {
            let absolute_pos = consumed + pos;
            let record_bytes = &self.buffer[consumed..absolute_pos];
            consumed = absolute_pos + 2;

            // 完整记录解码
            let record = String::from_utf8_lossy(record_bytes);
            for line in record.lines() {
                if let Some(payload) = line.strip_prefix("data:") {
                    let payload = payload.trim();
                    if payload == "[DONE]" {
                        events.push(ParsedEvent::Done);
                        continue;
                    }
                    if payload.is_empty() {
                        continue;
                    }
                    match Self::parse_payload(payload) {
                        Ok(parsed) => events.extend(parsed),
                        Err(err) => events.push(ParsedEvent::Error(err)),
                    }
                }
            }
        }

        if consumed > 0 {
            self.buffer.drain(..consumed);
        }
        events
    }

    /// 强制冲刷剩余缓冲
    #[allow(dead_code)]
    pub fn flush(&mut self) -> Vec<ParsedEvent> {
        let mut events = Vec::new();
        let remaining = std::mem::take(&mut self.buffer);
        let text = String::from_utf8_lossy(&remaining);
        for line in text.lines() {
            if let Some(payload) = line.strip_prefix("data:") {
                let payload = payload.trim();
                if payload == "[DONE]" || payload.is_empty() {
                    continue;
                }
                if let Ok(parsed) = Self::parse_payload(payload) {
                    events.extend(parsed);
                }
            }
        }
        events
    }

    /// 解析单条 JSON
    fn parse_payload(payload: &str) -> Result<Vec<ParsedEvent>, String> {
        let value: Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
        let mut events = Vec::new();

        // 顶层错误字段处理
        if let Some(err) = value.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            events.push(ParsedEvent::Error(msg));
            return Ok(events);
        }

        // 解析 choices 数组
        if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
            for choice in choices {
                let delta = choice.get("delta").or_else(|| choice.get("message"));
                if let Some(delta) = delta {
                    // 主文本内容
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        if !content.is_empty() {
                            events.push(ParsedEvent::Token(content.to_string()));
                        }
                    }
                    // 思维链内容(兼容多家 API 格式)
                    let thinking_text = delta
                        .get("reasoning_content")
                        .and_then(|v| v.as_str())
                        .or_else(|| delta.get("reasoning").and_then(|v| v.as_str()))
                        .or_else(|| delta.get("thinking").and_then(|v| v.as_str()))
                        .or_else(|| delta.get("thought").and_then(|v| v.as_str()));
                    if let Some(text) = thinking_text {
                        if !text.is_empty() {
                            events.push(ParsedEvent::Thinking(text.to_string()));
                        }
                    }
                    // 工具调用增量
                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                        for (idx, tc) in tool_calls.iter().enumerate() {
                            let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(idx as u64) as usize;
                            let id = tc.get("id").and_then(|v| v.as_str()).map(String::from);
                            let function = tc.get("function");
                            let name = function
                                .and_then(|f| f.get("name"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let arguments = function
                                .and_then(|f| f.get("arguments"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            events.push(ParsedEvent::ToolCallDelta {
                                index,
                                id,
                                name,
                                arguments,
                            });
                        }
                    }
                }
                // finish_reason 出现意味着该 choice 已结束
                let finish = choice
                    .get("finish_reason")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty() && *s != "null");
                if finish.is_some() {
                    events.push(ParsedEvent::Done);
                }
            }
        }

        Ok(events)
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

/// 查找连续两个换行字节
fn find_double_newline(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|w| w == [b'\n', b'\n'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_token() {
        let mut parser = SseParser::new();
        let chunk = b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        let events = parser.feed(chunk);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Token(s) => assert_eq!(s, "hi"),
            _ => panic!("expected token"),
        }
    }

    #[test]
    fn handles_partial_lines() {
        let mut parser = SseParser::new();
        parser.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hel");
        let events = parser.feed(b"lo\"}}]}\n\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Token(s) => assert_eq!(s, "hello"),
            _ => panic!("expected token"),
        }
    }

    #[test]
    fn detects_done() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: [DONE]\n\n");
        assert!(matches!(events[0], ParsedEvent::Done));
    }

    #[test]
    fn handles_split_multibyte() {
        // 中文字符 UTF-8 为 3 字节, 故意切在中间
        let mut parser = SseParser::new();
        let full = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n";
        let bytes = full.as_bytes();
        let split = 30;
        parser.feed(&bytes[..split]);
        let events = parser.feed(&bytes[split..]);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Token(s) => assert_eq!(s, "你好"),
            _ => panic!("expected token"),
        }
    }
}
