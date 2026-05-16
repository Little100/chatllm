<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import MessageList from './MessageList.vue'
import ChatInput from '@/components/input/ChatInput.vue'
import FileDropZone from '@/components/input/FileDropZone.vue'
import type { SendPayload } from '@/components/input/ChatInput.vue'
import { useStreamListener } from '@/composables/useStreamListener'
import { useStreamStore, initStreamStoreListener } from '@/stores/streamStore'
import { useI18n } from '@/composables/useI18n'
import { MessageSquarePlus } from 'lucide-vue-next'
import type { Message } from '@/types/chat'

const { t } = useI18n()
initStreamStoreListener()
const streamStore = useStreamStore()

const props = defineProps<{ sessionId: string | null }>()

const messages = ref<Message[]>([])
const versionMap = ref<Record<string, { total: number; current: number }>>({})
const currentModelConfigId = ref<string | null>(null)
const isDragging = ref(false)
const showDropChoice = ref(false)
const pendingDropPaths = ref<string[]>([])
const chatInputRef = ref<InstanceType<typeof ChatInput> | null>(null)
let unlistenDragDrop: (() => void) | null = null

onMounted(async () => {
  try {
    const configs = await invoke<{ id: string }[]>('list_model_configs')
    if (configs.length > 0 && !currentModelConfigId.value) {
      currentModelConfigId.value = configs[0].id
    }
  } catch (_) {}

  const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === 'enter') {
      isDragging.value = true
    } else if (event.payload.type === 'leave') {
      isDragging.value = false
    } else if (event.payload.type === 'drop') {
      isDragging.value = false
      pendingDropPaths.value = event.payload.paths
      showDropChoice.value = true
    }
  })
  unlistenDragDrop = unlisten
})
const {
  streamingContent,
  thinkingContent,
  toolCalls,
  toolExecs,
  rounds,
  currentRound,
  isStreaming,
  isInterrupted,
  error,
  activeStreamId,
  startStream,
  startRegenerate,
  cancelStream,
  confirmTool,
  finishStream,
  detach,
} = useStreamListener()

async function loadMessages() {
  if (!props.sessionId) {
    messages.value = []
    versionMap.value = {}
    return
  }
  try {
    messages.value = await invoke<Message[]>('list_messages', { sessionId: props.sessionId })
    await buildVersionMap()
  } catch (_) {
    messages.value = []
    versionMap.value = {}
  }
}

async function buildVersionMap() {
  const map: Record<string, { total: number; current: number }> = {}
  for (const msg of messages.value) {
    if (!msg.parent_id) continue
    if (map[msg.parent_id]) continue
    try {
      const versions = await invoke<Message[]>('get_message_versions', { parentId: msg.parent_id })
      map[msg.parent_id] = {
        total: versions.length,
        current: msg.version,
      }
    } catch (_) {
      map[msg.parent_id] = { total: 1, current: 1 }
    }
  }
  versionMap.value = map
}

async function handleSwitchVersion(parentId: string, version: number) {
  try {
    const msg = await invoke<Message>('switch_version', { parentId, version })
    const idx = messages.value.findIndex(m => m.parent_id === parentId)
    if (idx !== -1) {
      messages.value[idx] = msg
      versionMap.value[parentId] = {
        ...versionMap.value[parentId],
        current: version,
      }
    }
  } catch (_) {}
}

async function handleEditMessage(id: string, content: string) {
  if (!props.sessionId) return
  const idx = messages.value.findIndex(m => m.id === id)
  if (idx === -1) return
  try {
    await invoke('update_message', { id, content })
    messages.value[idx] = { ...messages.value[idx], content }

    if (messages.value[idx].role !== 'user') return

    const assistant = messages.value.find(m => m.parent_id === id)
    if (!assistant) return
    if (!currentModelConfigId.value) return

    const oldAssistantId = assistant.id
    messages.value = messages.value.filter(m => m.id !== oldAssistantId)
    const completed = await startRegenerate(oldAssistantId, currentModelConfigId.value)
    if (!completed) return
    finishStream()
    try {
      await invoke('delete_message', { id: oldAssistantId })
    } catch (_) {}
    await loadMessages()
  } catch (e) {
    console.error(e)
  }
}

async function handleDeleteMessage(id: string) {
  try {
    await invoke('delete_message', { id })
    messages.value = messages.value.filter(m => m.id !== id)
    await buildVersionMap()
  } catch (e) {
    console.error(e)
  }
}

async function handleCopyMessage(_id: string, content: string) {
  try {
    await writeText(content)
  } catch (e) {
    console.error(e)
  }
}

async function handleRegenerate(messageId: string) {
  if (!props.sessionId || !currentModelConfigId.value) return
  messages.value = messages.value.filter(m => m.id !== messageId)
  const completed = await startRegenerate(messageId, currentModelConfigId.value)
  if (!completed) return
  finishStream()
  try {
    await invoke('delete_message', { id: messageId })
  } catch (_) {}
  await loadMessages()
}

// 轮询机制: 当前 session 有后台活跃流时定期刷新消息
let pollTimer: ReturnType<typeof setInterval> | null = null

function startPolling() {
  stopPolling()
  pollTimer = setInterval(async () => {
    if (!props.sessionId || !streamStore.isActive(props.sessionId)) {
      stopPolling()
      await loadMessages()
      return
    }
    await loadMessages()
  }, 800)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

watch(() => props.sessionId, async (id, oldId) => {
  if (isStreaming.value && oldId) {
    streamStore.register(oldId, activeStreamId.value || '')
    detach()
  }
  finishStream()
  error.value = null
  isInterrupted.value = false
  messages.value = []
  versionMap.value = {}
  stopPolling()
  if (!id) return
  streamStore.consumeFinished(id)
  await loadMessages()
  if (streamStore.isActive(id)) {
    startPolling()
  }
}, { immediate: true })

// 后台流完成时, 如果当前正在看该 session 则自动刷新
const unlistenFinished = listen<{ session_id: string }>('stream-finished', async (event) => {
  if (event.payload.session_id === props.sessionId) {
    stopPolling()
    await loadMessages()
  }
})
const unlistenInterrupted = listen<{ session_id: string }>('stream-interrupted', async (event) => {
  if (event.payload.session_id === props.sessionId) {
    stopPolling()
    await loadMessages()
  }
})
onUnmounted(() => {
  stopPolling()
  unlistenDragDrop?.()
  unlistenFinished.then(fn => fn())
  unlistenInterrupted.then(fn => fn())
})

async function handleSend(payload: SendPayload) {
  const { text, attachments, modelConfigId } = payload
  if (!props.sessionId || !text.trim()) return
  currentModelConfigId.value = modelConfigId
  error.value = null
  messages.value.push({
    id: crypto.randomUUID(),
    session_id: props.sessionId!,
    role: 'user',
    content: text.trim(),
    version: 1,
    parent_id: null,
    created_at: new Date().toISOString(),
    token_count: 0,
    error: null,
    thinking: null,
    attachments: attachments.length > 0 ? attachments : undefined,
  })
  await nextTick()
  const sid = props.sessionId!
  streamStore.register(sid, 'pending')
  const completed = await startStream(sid, text.trim(), modelConfigId, attachments)
  if (!completed) return
  streamStore.unregister(sid)
  finishStream()
  await loadMessages()
}

function handleCancel() {
  cancelStream(activeStreamId.value || undefined)
}

function handleGlobalDrop(fileList: FileList) {
  isDragging.value = false
  chatInputRef.value?.addFiles(fileList)
}

function handleDropChoice(includePath: boolean) {
  showDropChoice.value = false
  chatInputRef.value?.addFilesFromPaths(pendingDropPaths.value, includePath)
  pendingDropPaths.value = []
}
</script>

<template>
  <div
    class="relative flex h-full flex-col"
    @dragenter.prevent
    @dragover.prevent
    @dragleave.prevent
    @drop.prevent
  >
    <FileDropZone v-if="isDragging" @drop="handleGlobalDrop" @cancel="isDragging = false" />

    <!-- 拖拽选择弹窗 -->
    <div
      v-if="showDropChoice"
      class="absolute inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      @click.self="showDropChoice = false; pendingDropPaths = []"
    >
      <div class="flex flex-col gap-3 rounded-lg border border-border bg-background p-5 shadow-lg max-w-sm">
        <p class="text-sm font-medium text-foreground">{{ t('dropzone.choiceTitle') }}</p>
        <button
          class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
          @click="handleDropChoice(false)"
        >{{ t('dropzone.fileOnly') }}</button>
        <button
          class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
          @click="handleDropChoice(true)"
        >{{ t('dropzone.fileWithPath') }}</button>
      </div>
    </div>

    <!-- 空状态 -->
    <template v-if="!sessionId">
      <div class="flex flex-1 flex-col items-center justify-center gap-4 text-muted-foreground">
        <MessageSquarePlus class="h-12 w-12 opacity-30" />
        <p class="text-sm">{{ t('chat.empty') }}</p>
      </div>
    </template>

    <template v-else>
      <!-- 消息列表 -->
      <MessageList
        :messages="messages"
        :streaming-content="streamingContent"
        :thinking-content="thinkingContent"
        :tool-calls="toolCalls"
        :tool-execs="toolExecs"
        :rounds="rounds"
        :current-round="currentRound"
        :is-streaming="isStreaming"
        :is-interrupted="isInterrupted"
        :error="error"
        :version-map="versionMap"
        @switch-version="handleSwitchVersion"
        @edit-message="handleEditMessage"
        @delete-message="handleDeleteMessage"
        @copy-message="handleCopyMessage"
        @regenerate-message="handleRegenerate"
        @confirm-tool="(id, approved) => confirmTool(id, approved)"
      />

      <!-- 输入区 -->
      <ChatInput
        ref="chatInputRef"
        :is-streaming="isStreaming"
        :disabled="!sessionId"
        @send="handleSend"
        @cancel="handleCancel"
      />
    </template>
  </div>
</template>
