/// API 日志(匹配后端 ApiLog)
export interface ApiLog {
  id: string
  session_id: string | null
  model: string | null
  provider: string | null
  request_body: string | null
  response_body: string | null
  status_code: number | null
  latency_ms: number | null
  created_at: string
}
