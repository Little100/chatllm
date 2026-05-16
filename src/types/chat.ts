/// 消息角色类型
export type MessageRole = 'user' | 'assistant' | 'system'

/// 聊天消息(匹配后端 ChatMessage)
export interface Message {
  id: string
  session_id: string
  role: string
  content: string
  version: number
  parent_id: string | null
  created_at: string
  token_count: number
  error: string | null
  thinking: string | null
  attachments?: string[]
}

/// 消息发送参数
export interface SendMessageParams {
  content: string
  sessionId: string
  modelConfigId?: string
}
