import { ref } from 'vue'
import { invoke, Channel } from '@tauri-apps/api/core'
import type { StreamEvent } from '@/lib/sse-types'
import { useStreamStore } from '@/stores/streamStore'

export interface ToolCallInfo {
  index: number
  name: string
  arguments: string
}

export interface ToolExecInfo {
  name: string
  arguments: string
  status: 'running' | 'success' | 'failed' | 'awaiting_confirm'
  resultPreview?: string
  toolCallId?: string
}

/// 单轮工具调用片段, snake_case 与持久化 marker 对齐
export interface RoundToolCall {
  name: string
  arguments: string
}

/// 单轮工具执行结果, status 字段在流式与历史区共用
export interface RoundToolExec {
  name: string
  arguments: string
  status: 'running' | 'success' | 'failed' | 'awaiting_confirm'
  result_preview?: string
}

/// 单轮完整数据, 用于流式聚合与持久化 marker 解析
export interface RoundData {
  thinking: string
  content: string
  tool_calls: RoundToolCall[]
  tool_execs: RoundToolExec[]
}

function emptyRound(): RoundData {
  return { thinking: '', content: '', tool_calls: [], tool_execs: [] }
}

function isRoundEmpty(r: RoundData): boolean {
  return !r.thinking && !r.content && r.tool_calls.length === 0 && r.tool_execs.length === 0
}

/// 模块级自增 epoch, 用于隔离不同 stream 的 channel 闭包
let streamEpoch = 0

/// 后台 channel 保活表: sessionId -> channel 引用
const backgroundChannels = new Map<string, Channel<StreamEvent>>()

export function useStreamListener() {
  const streamingContent = ref('')
  const thinkingContent = ref('')
  const toolCalls = ref<ToolCallInfo[]>([])
  const toolExecs = ref<ToolExecInfo[]>([])
  const rounds = ref<RoundData[]>([])
  const currentRound = ref<RoundData>(emptyRound())
  const isStreaming = ref(false)
  const isInterrupted = ref(false)
  const error = ref<string | null>(null)
  const currentMessageId = ref<string | null>(null)
  const activeStreamId = ref<string | null>(null)

  function reset() {
    streamingContent.value = ''
    thinkingContent.value = ''
    toolCalls.value = []
    toolExecs.value = []
    rounds.value = []
    currentRound.value = emptyRound()
    error.value = null
    isInterrupted.value = false
    currentMessageId.value = null
  }

  /// 流终止前归档当前轮, 兜底缺失的 round_end
  function flushCurrentRound() {
    if (!isRoundEmpty(currentRound.value)) {
      rounds.value.push(currentRound.value)
      currentRound.value = emptyRound()
    }
  }

  async function startStream(
    sessionId: string,
    content: string,
    modelConfigId: string,
    attachments: string[] = []
  ): Promise<boolean> {
    return runStream({
      cmd: 'send_message',
      args: { sessionId, content, modelConfigId, attachments },
      sessionId,
    })
  }

  async function startRegenerate(messageId: string, modelConfigId: string): Promise<boolean> {
    return runStream({
      cmd: 'regenerate',
      args: { messageId, modelConfigId, defaultSystemPrompt: null },
    })
  }

  async function runStream(opts: { cmd: string; args: Record<string, unknown>; sessionId?: string }): Promise<boolean> {
    isStreaming.value = true
    reset()

    activeStreamId.value = 'pending'
    const myEpoch = ++streamEpoch
    const sessionId = opts.sessionId || ''

    let resolveDone: (detached: boolean) => void = () => {}
    const donePromise = new Promise<boolean>((res) => { resolveDone = res })

    const channel = new Channel<StreamEvent>()
    // 保持 channel 引用防止 GC
    if (sessionId) {
      backgroundChannels.set(sessionId, channel)
    }

    const streamStore = useStreamStore()

    channel.onmessage = (event) => {
      // 终结事件无论 epoch 是否匹配都需要处理(清理后台引用)
      if (event.type === 'done' || event.type === 'interrupted' || event.type === 'error') {
        if (sessionId) {
          backgroundChannels.delete(sessionId)
          streamStore.markFinished(sessionId)
        }
        // epoch 不匹配时只做清理, 不更新 UI 状态
        if (myEpoch !== streamEpoch) {
          resolveDone(true)
          return
        }
      }

      // 非当前活跃 epoch 的普通事件直接丢弃
      if (myEpoch !== streamEpoch) return

      switch (event.type) {
        case 'token':
          streamingContent.value += event.content
          currentRound.value.content += event.content
          break
        case 'thinking':
          thinkingContent.value += event.content
          currentRound.value.thinking += event.content
          break
        case 'tool_call': {
          const existing = toolCalls.value.find(tc => tc.index === event.index)
          if (existing) {
            if (event.name) existing.name = event.name
            existing.arguments += event.arguments
          } else {
            toolCalls.value.push({
              index: event.index,
              name: event.name,
              arguments: event.arguments,
            })
          }
          if (event.name) {
            const last = currentRound.value.tool_calls[currentRound.value.tool_calls.length - 1]
            if (last && last.name === event.name && !last.arguments) {
              last.arguments += event.arguments
            } else {
              currentRound.value.tool_calls.push({
                name: event.name,
                arguments: event.arguments,
              })
            }
          } else {
            const last = currentRound.value.tool_calls[currentRound.value.tool_calls.length - 1]
            if (last) last.arguments += event.arguments
          }
          break
        }
        case 'tool_exec_start': {
          const pending = toolExecs.value.find(
            t => t.name === event.name && t.status === 'awaiting_confirm'
          )
          if (pending) {
            pending.status = 'running'
          } else {
            toolExecs.value.push({
              name: event.name,
              arguments: event.arguments,
              status: 'running',
            })
          }
          const pendingRound = currentRound.value.tool_execs.find(
            t => t.name === event.name && t.status === 'awaiting_confirm'
          )
          if (pendingRound) {
            pendingRound.status = 'running'
          } else {
            currentRound.value.tool_execs.push({
              name: event.name,
              arguments: event.arguments,
              status: 'running',
            })
          }
          break
        }
        case 'tool_confirm':
          toolExecs.value.push({
            name: event.name,
            arguments: event.arguments,
            status: 'awaiting_confirm',
            toolCallId: event.tool_call_id,
          })
          currentRound.value.tool_execs.push({
            name: event.name,
            arguments: event.arguments,
            status: 'awaiting_confirm',
          })
          break
        case 'tool_exec_done': {
          const item = [...toolExecs.value].reverse().find(
            t => t.name === event.name && t.status === 'running'
          )
          if (item) {
            item.status = event.success ? 'success' : 'failed'
            item.resultPreview = event.content
          }
          const roundItem = [...currentRound.value.tool_execs].reverse().find(
            t => t.name === event.name && t.status === 'running'
          )
          if (roundItem) {
            roundItem.status = event.success ? 'success' : 'failed'
            roundItem.result_preview = event.content
          }
          break
        }
        case 'round_end':
          rounds.value.push(currentRound.value)
          currentRound.value = emptyRound()
          break
        case 'done':
          flushCurrentRound()
          currentMessageId.value = event.message_id
          activeStreamId.value = null
          resolveDone(false)
          break
        case 'interrupted':
          flushCurrentRound()
          currentMessageId.value = event.message_id
          isInterrupted.value = true
          activeStreamId.value = null
          resolveDone(false)
          break
        case 'error':
          error.value = event.message
          activeStreamId.value = null
          resolveDone(false)
          break
      }
    }

    try {
      const msgId = await invoke<string>(opts.cmd, { ...opts.args, channel })
      currentMessageId.value = msgId
      if (activeStreamId.value !== null) {
        activeStreamId.value = msgId
      }
    } catch (e) {
      error.value = String(e)
      isStreaming.value = false
      activeStreamId.value = null
      if (sessionId) {
        backgroundChannels.delete(sessionId)
      }
      resolveDone(false)
      return false
    }

    const detached = await donePromise
    return !detached
  }

  /// 由调用方在 loadMessages 完成后翻转流状态, 避免气泡闪烁
  function finishStream() {
    isStreaming.value = false
    streamingContent.value = ''
    thinkingContent.value = ''
    toolCalls.value = []
    toolExecs.value = []
    rounds.value = []
    currentRound.value = emptyRound()
  }

  // 断开 UI 与当前流的关联, 后台继续生成不中断
  // channel 保留在 backgroundChannels 中防止 GC, 后端继续发送
  // donePromise 不 resolve, handleSend 会一直挂起(不影响新 session 操作)
  function detach(): { streamId: string | null } {
    const id = activeStreamId.value
    streamEpoch++
    isStreaming.value = false
    activeStreamId.value = null
    return { streamId: id === 'pending' ? null : id }
  }

  async function cancelStream(streamId?: string) {
    const id = streamId || activeStreamId.value
    if (!id || id === 'pending') return
    streamEpoch++
    try {
      await invoke('cancel_stream', { streamId: id })
    } catch (e) {
      console.error('cancel_stream failed', e)
    }
    isInterrupted.value = true
    isStreaming.value = false
    activeStreamId.value = null
  }

  async function confirmTool(toolCallId: string, approved: boolean) {
    try {
      await invoke('confirm_tool', { toolCallId, approved })
    } catch (e) {
      console.error('confirm_tool failed', e)
    }
    const item = toolExecs.value.find(t => t.toolCallId === toolCallId)
    if (item) {
      item.status = approved ? 'running' : 'failed'
      if (!approved) item.resultPreview = '用户拒绝执行'
    }
  }

  return {
    streamingContent,
    thinkingContent,
    toolCalls,
    toolExecs,
    rounds,
    currentRound,
    isStreaming,
    isInterrupted,
    error,
    currentMessageId,
    activeStreamId,
    startStream,
    startRegenerate,
    cancelStream,
    confirmTool,
    finishStream,
    detach,
  }
}
