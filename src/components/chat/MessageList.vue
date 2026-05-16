<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed, watch } from 'vue'
import { ChevronDown, ChevronRight, AlertCircle } from 'lucide-vue-next'
import MessageItem from './MessageItem.vue'
import ToolCallBlock from './ToolCallBlock.vue'
import StreamingIndicator from './StreamingIndicator.vue'
import { useAutoScroll } from '@/composables/useAutoScroll'
import { useAvatar } from '@/composables/useAvatar'
import { useI18n } from '@/composables/useI18n'
import { renderMarkdown } from '@/lib/markdown'
import { Bot } from 'lucide-vue-next'
import type { Message } from '@/types/chat'
import type { ToolCallInfo, ToolExecInfo, RoundData } from '@/composables/useStreamListener'

const { t } = useI18n()
const { llmAvatar } = useAvatar()

const props = defineProps<{
  messages: Message[]
  streamingContent: string
  thinkingContent: string
  toolCalls: ToolCallInfo[]
  toolExecs?: ToolExecInfo[]
  rounds: RoundData[]
  currentRound: RoundData
  isStreaming: boolean
  isInterrupted?: boolean
  error?: string | null
  versionMap: Record<string, { total: number; current: number }>
}>()

const emit = defineEmits<{
  switchVersion: [parentId: string, version: number]
  editMessage: [id: string, content: string]
  deleteMessage: [id: string]
  copyMessage: [id: string, content: string]
  regenerateMessage: [id: string]
  confirmTool: [toolCallId: string, approved: boolean]
}>()

const containerRef = ref<HTMLElement | null>(null)
const { startObserving, scrollToBottom } = useAutoScroll(containerRef)

const llmAvatarSrc = computed(() => llmAvatar.value ? `data:image/png;base64,${llmAvatar.value}` : null)

onMounted(() => {
  startObserving()
  scrollToBottom()
})

function getVersionInfo(msg: Message) {
  if (!msg.parent_id || !props.versionMap[msg.parent_id]) {
    return { total: 1, current: 1 }
  }
  return props.versionMap[msg.parent_id]
}

/// 历史轮 thinking 折叠态, 索引为 rounds 下标
const thinkingExpandedMap = ref<Record<number, boolean>>({})
/// 当前轮 thinking 折叠态
const currentThinkingExpanded = ref(false)

function toggleRoundThinking(idx: number) {
  thinkingExpandedMap.value[idx] = !thinkingExpandedMap.value[idx]
}

/// 历史轮的 markdown 渲染缓存, 仅在 rounds 数组追加时计算
const roundsHtml = computed(() => {
  return props.rounds.map(r => r.content ? renderMarkdown(r.content) : '')
})

// 当前轮 content 节流渲染, 60ms 合并多次 token
const currentRoundHtml = ref('')
let renderTimer: ReturnType<typeof setTimeout> | null = null
let lastRenderedSource = ''

function flushCurrentRender() {
  if (renderTimer) {
    clearTimeout(renderTimer)
    renderTimer = null
  }
  const src = props.currentRound.content
  if (src !== lastRenderedSource) {
    lastRenderedSource = src
    currentRoundHtml.value = src ? renderMarkdown(src) : ''
  }
}

watch(
  () => props.currentRound.content,
  (val) => {
    if (!val) {
      if (renderTimer) {
        clearTimeout(renderTimer)
        renderTimer = null
      }
      lastRenderedSource = ''
      currentRoundHtml.value = ''
      return
    }
    if (renderTimer) return
    renderTimer = setTimeout(() => {
      renderTimer = null
      if (props.currentRound.content !== lastRenderedSource) {
        lastRenderedSource = props.currentRound.content
        currentRoundHtml.value = renderMarkdown(props.currentRound.content)
      }
    }, 60)
  },
)

// 流终止时立即落盘
watch(
  () => props.isStreaming,
  (val) => {
    if (!val) flushCurrentRender()
  },
)

onBeforeUnmount(() => {
  if (renderTimer) clearTimeout(renderTimer)
})

/// 流式气泡可见性, 任意一轮或当前轮有内容即显示
const hasStreamPayload = computed(() => {
  if (props.rounds.length > 0) return true
  const c = props.currentRound
  return !!(c.thinking || c.content || c.tool_calls.length || c.tool_execs.length)
})
</script>

<template>
  <div ref="containerRef" class="flex-1 overflow-y-auto px-4 py-6">
    <div class="mx-auto max-w-3xl space-y-4">
      <MessageItem
        v-for="msg in messages"
        :key="msg.id"
        :message="msg"
        :total-versions="getVersionInfo(msg).total"
        :current-version="getVersionInfo(msg).current"
        @switch-version="(parentId, version) => emit('switchVersion', parentId, version)"
        @edit-message="(id, content) => emit('editMessage', id, content)"
        @delete-message="(id) => emit('deleteMessage', id)"
        @copy-message="(id, content) => emit('copyMessage', id, content)"
        @regenerate-message="(id) => emit('regenerateMessage', id)"
      />
      <!-- 流式输出中 -->
      <div v-if="isStreaming && hasStreamPayload" class="flex gap-3">
        <div class="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full text-xs font-medium" :class="!llmAvatarSrc && 'bg-primary text-primary-foreground'">
          <img v-if="llmAvatarSrc" :src="llmAvatarSrc" class="h-full w-full object-cover" />
          <Bot v-else class="h-3.5 w-3.5" />
        </div>
        <div class="max-w-[80%] min-w-0 rounded-lg bg-muted px-4 py-3 text-sm leading-relaxed">
          <!-- 历史轮顺序渲染 -->
          <template v-for="(round, idx) in rounds" :key="`r-${idx}`">
            <div v-if="round.thinking" class="mb-2">
              <button
                class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                @click="toggleRoundThinking(idx)"
              >
                <ChevronDown v-if="thinkingExpandedMap[idx]" class="h-3 w-3" />
                <ChevronRight v-else class="h-3 w-3" />
                <span>{{ t('chat.thinking') }}</span>
              </button>
              <div
                v-if="thinkingExpandedMap[idx]"
                class="mt-1 rounded border border-border/50 bg-background/50 px-3 py-2 text-xs text-muted-foreground leading-relaxed whitespace-pre-wrap"
              >
                {{ round.thinking }}
              </div>
            </div>
            <ToolCallBlock
              v-for="(exec, ei) in round.tool_execs"
              :key="`r-${idx}-e-${ei}`"
              :name="exec.name"
              :arguments="exec.arguments"
              :status="exec.status"
              :result-preview="exec.result_preview"
            />
            <div v-if="round.content" v-html="roundsHtml[idx]" class="message-content" />
          </template>

          <!-- 当前轮 -->
          <div v-if="currentRound.thinking" class="mb-2">
            <button
              class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
              @click="currentThinkingExpanded = !currentThinkingExpanded"
            >
              <ChevronDown v-if="currentThinkingExpanded" class="h-3 w-3" />
              <ChevronRight v-else class="h-3 w-3" />
              <span>{{ t('chat.thinking') }}</span>
            </button>
            <div
              v-if="currentThinkingExpanded"
              class="mt-1 rounded border border-border/50 bg-background/50 px-3 py-2 text-xs text-muted-foreground leading-relaxed whitespace-pre-wrap"
            >
              {{ currentRound.thinking }}
            </div>
          </div>
          <ToolCallBlock
            v-for="(exec, ei) in currentRound.tool_execs"
            :key="`cur-e-${ei}`"
            :name="exec.name"
            :arguments="exec.arguments"
            :status="exec.status"
            :result-preview="exec.result_preview"
            @confirm="emit('confirmTool', (toolExecs?.find(t => t.name === exec.name && t.status === 'awaiting_confirm')?.toolCallId) || '', true)"
            @reject="emit('confirmTool', (toolExecs?.find(t => t.name === exec.name && t.status === 'awaiting_confirm')?.toolCallId) || '', false)"
          />
          <div v-if="currentRound.content" v-html="currentRoundHtml" class="message-content" />
          <StreamingIndicator />
        </div>
      </div>
      <div v-else-if="isStreaming" class="flex gap-3">
        <div class="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full text-xs font-medium" :class="!llmAvatarSrc && 'bg-primary text-primary-foreground'">
          <img v-if="llmAvatarSrc" :src="llmAvatarSrc" class="h-full w-full object-cover" />
          <Bot v-else class="h-3.5 w-3.5" />
        </div>
        <div class="flex-1 rounded-lg bg-muted px-4 py-3">
          <StreamingIndicator />
        </div>
      </div>
      <!-- 错误提示作为 AI 消息气泡 -->
      <div v-if="error && !isStreaming" class="flex gap-3">
        <div class="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full text-xs font-medium" :class="!llmAvatarSrc && 'bg-primary text-primary-foreground'">
          <img v-if="llmAvatarSrc" :src="llmAvatarSrc" class="h-full w-full object-cover" />
          <Bot v-else class="h-3.5 w-3.5" />
        </div>
        <div class="max-w-[80%] rounded-lg border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm leading-relaxed">
          <div class="flex items-center gap-2 mb-1 text-xs font-medium text-destructive">
            <AlertCircle class="h-3.5 w-3.5" />
            <span>请求失败</span>
          </div>
          <div class="text-destructive/90 whitespace-pre-wrap break-words">{{ error }}</div>
        </div>
      </div>
      <!-- 打断提示 -->
      <div v-if="isInterrupted" class="mx-auto flex items-center gap-2 rounded-md bg-amber-500/10 border border-amber-500/30 px-3 py-1.5 text-xs text-amber-700 dark:text-amber-400 w-fit">
        <AlertCircle class="h-3.5 w-3.5" />
        <span>{{ t('chat.interrupted') || '已打断生成' }}</span>
      </div>
    </div>
  </div>
</template>
