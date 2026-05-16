<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, ChevronRight } from 'lucide-vue-next'
import { renderMarkdown } from '@/lib/markdown'
import ToolCallBlock from './ToolCallBlock.vue'
import { useI18n } from '@/composables/useI18n'
import type { RoundData } from '@/composables/useStreamListener'

const { t } = useI18n()

const props = defineProps<{
  content: string
  isUser: boolean
}>()

const TOOL_CALLS_MARKER = /\n\n<!-- tool_calls:(.*?) -->$/s
const CHAT_ROUNDS_MARKER = /\n\n<!-- chat_rounds:(.*?) -->$/s

/// 优先解析 chat_rounds, 命中即按轮渲染
const parsedRounds = computed<RoundData[]>(() => {
  if (props.isUser) return []
  const match = props.content.match(CHAT_ROUNDS_MARKER)
  if (!match) return []
  try {
    const arr = JSON.parse(match[1]) as RoundData[]
    return Array.isArray(arr) ? arr : []
  } catch {
    return []
  }
})

const hasRounds = computed(() => parsedRounds.value.length > 0)

/// 旧格式 tool_calls 兼容, 仅在无 chat_rounds 时启用
const parsedToolCalls = computed<{ name: string; arguments: string }[]>(() => {
  if (props.isUser || hasRounds.value) return []
  const match = props.content.match(TOOL_CALLS_MARKER)
  if (!match) return []
  try {
    return JSON.parse(match[1])
  } catch {
    return []
  }
})

const cleanedContent = computed(() => {
  return props.content
    .replace(CHAT_ROUNDS_MARKER, '')
    .replace(TOOL_CALLS_MARKER, '')
})

const renderedHtml = computed(() => {
  if (props.isUser || hasRounds.value) return ''
  return renderMarkdown(cleanedContent.value)
})

/// 历史轮各自的 markdown 缓存
const renderedRoundsHtml = computed(() => {
  return parsedRounds.value.map(r => r.content ? renderMarkdown(r.content) : '')
})

/// 历史轮 thinking 折叠态
const thinkingExpandedMap = ref<Record<number, boolean>>({})

function toggleThinking(idx: number) {
  thinkingExpandedMap.value[idx] = !thinkingExpandedMap.value[idx]
}
</script>

<template>
  <div>
    <!-- 用户消息保持纯文本 -->
    <pre v-if="isUser" class="message-content user-text whitespace-pre-wrap wrap-break-word font-sans">{{ cleanedContent }}</pre>

    <!-- 新格式: 按轮渲染 -->
    <template v-else-if="hasRounds">
      <template v-for="(round, idx) in parsedRounds" :key="idx">
        <div v-if="round.thinking" class="mb-2">
          <button
            class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
            @click="toggleThinking(idx)"
          >
            <ChevronDown v-if="thinkingExpandedMap[idx]" class="h-3 w-3" />
            <ChevronRight v-else class="h-3 w-3" />
            <span>{{ t('chat.thinking') }}</span>
          </button>
          <div
            v-if="thinkingExpandedMap[idx]"
            class="mt-1 rounded border border-border/50 bg-background/50 px-3 py-2 text-xs text-muted-foreground leading-relaxed whitespace-pre-wrap max-h-64 overflow-y-auto"
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
        <!-- 兜底: 仅有 tool_calls 而无 tool_execs(被打断时) -->
        <template v-if="round.tool_execs.length === 0 && round.tool_calls.length">
          <ToolCallBlock
            v-for="(tc, ci) in round.tool_calls"
            :key="`r-${idx}-c-${ci}`"
            :name="tc.name"
            :arguments="tc.arguments"
          />
        </template>
        <div v-if="round.content" v-html="renderedRoundsHtml[idx]" class="message-content" />
      </template>
    </template>

    <!-- 旧格式: tool_calls marker + 单段 markdown -->
    <template v-else>
      <ToolCallBlock
        v-for="(tc, idx) in parsedToolCalls"
        :key="idx"
        :name="tc.name"
        :arguments="tc.arguments"
      />
      <div class="message-content" v-html="renderedHtml" />
    </template>
  </div>
</template>

<style>
.message-content.user-text {
  margin: 0;
  background: transparent;
  padding: 0;
}

.message-content pre.hljs-code-block {
  position: relative;
  margin: 0.75rem 0;
  padding: 1rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  font-size: 0.8rem;
  line-height: 1.5;
}

.message-content pre.hljs-code-block code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

.message-content p {
  margin: 0.25rem 0;
}

.message-content ul,
.message-content ol {
  padding-left: 1.25rem;
  margin: 0.5rem 0;
}

.message-content table {
  border-collapse: collapse;
  margin: 0.5rem 0;
  font-size: 0.8rem;
}

.message-content th,
.message-content td {
  border: 1px solid var(--color-border);
  padding: 0.375rem 0.75rem;
}

.message-content blockquote {
  border-left: 3px solid var(--color-border);
  padding-left: 0.75rem;
  margin: 0.5rem 0;
  color: var(--color-muted-foreground);
}

.message-content a {
  color: var(--color-primary);
  text-decoration: underline;
}

.message-content .katex-block {
  margin: 0.75rem 0;
  overflow-x: auto;
}

.message-content {
  overflow-wrap: break-word;
  word-break: break-word;
}
</style>
