/// 会话(匹配后端 Session)
export interface Session {
  id: string
  title: string
  model_config_id: string | null
  created_at: string
  updated_at: string
  pinned: boolean
  archived: boolean
}
