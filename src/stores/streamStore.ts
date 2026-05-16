import { reactive } from 'vue'
import { listen } from '@tauri-apps/api/event'

export interface BackgroundStream {
  streamId: string
  sessionId: string
  startedAt: number
}

// 全局活跃后台流注册表
const activeStreams = reactive<Map<string, BackgroundStream>>(new Map())

// 已完成但前端尚未刷新的 session 集合
const finishedSessions = reactive<Set<string>>(new Set())

export function useStreamStore() {
  function register(sessionId: string, streamId: string) {
    activeStreams.set(sessionId, { streamId, sessionId, startedAt: Date.now() })
    finishedSessions.delete(sessionId)
  }

  function unregister(sessionId: string) {
    activeStreams.delete(sessionId)
  }

  function isActive(sessionId: string): boolean {
    return activeStreams.has(sessionId)
  }

  function getStreamId(sessionId: string): string | null {
    return activeStreams.get(sessionId)?.streamId ?? null
  }

  function markFinished(sessionId: string) {
    activeStreams.delete(sessionId)
    finishedSessions.add(sessionId)
  }

  function consumeFinished(sessionId: string): boolean {
    if (finishedSessions.has(sessionId)) {
      finishedSessions.delete(sessionId)
      return true
    }
    return false
  }

  function hasActiveStreams(): boolean {
    return activeStreams.size > 0
  }

  function getActiveSessions(): string[] {
    return [...activeStreams.keys()]
  }

  return {
    activeStreams,
    finishedSessions,
    register,
    unregister,
    isActive,
    getStreamId,
    markFinished,
    consumeFinished,
    hasActiveStreams,
    getActiveSessions,
  }
}

// 监听后端 stream-finished 事件, 标记对应 session 已完成
let listenerInitialized = false
export function initStreamStoreListener() {
  if (listenerInitialized) return
  listenerInitialized = true
  const store = useStreamStore()
  listen<{ session_id: string; message_id: string }>('stream-finished', (event) => {
    store.markFinished(event.payload.session_id)
  })
  listen<{ session_id: string; message_id: string }>('stream-interrupted', (event) => {
    store.markFinished(event.payload.session_id)
  })
}
