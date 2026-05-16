/// 模型配置(匹配后端 ModelConfig)
export interface ModelConfig {
  id: string
  name: string
  provider: string
  model: string
  base_url: string | null
  api_key_name: string | null
  api_key: string | null
  max_tokens: number
  context_window: number
  temperature: number
  top_p: number
  temperature_enabled: boolean
  top_p_enabled: boolean
  system_prompt: string | null
  reasoning_effort: string | null
  passback_reasoning: boolean
  retry_count: number
  created_at: string
  updated_at: string
}
