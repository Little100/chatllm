export type StreamEvent =
  | { type: 'token'; content: string }
  | { type: 'thinking'; content: string }
  | { type: 'tool_call'; index: number; name: string; arguments: string }
  | { type: 'tool_confirm'; name: string; arguments: string; tool_call_id: string }
  | { type: 'tool_exec_start'; name: string; arguments: string }
  | { type: 'tool_exec_done'; name: string; success: boolean; content: string }
  | { type: 'round_end'; round_index: number }
  | { type: 'done'; message_id: string }
  | { type: 'interrupted'; message_id: string }
  | { type: 'error'; message: string }
