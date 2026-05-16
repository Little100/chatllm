<script setup lang="ts">
import { ref, computed } from 'vue'
import { ChevronDown, ChevronRight, Wrench, Loader2, Check, X, ShieldAlert } from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  name: string
  arguments: string
  status?: 'pending' | 'running' | 'success' | 'failed' | 'awaiting_confirm'
  resultPreview?: string
}>()

const emit = defineEmits<{
  confirm: []
  reject: []
}>()

const expanded = ref(false)
const needsConfirm = computed(() => props.status === 'awaiting_confirm')

const formattedArgs = computed(() => {
  try {
    const parsed = JSON.parse(props.arguments)
    return JSON.stringify(parsed, null, 2)
  } catch {
    return props.arguments
  }
})

const statusLabel = computed(() => {
  switch (props.status) {
    case 'awaiting_confirm': return '等待确认'
    case 'running': return t.value('chat.toolRunning') || '执行中'
    case 'success': return t.value('chat.toolSuccess') || '完成'
    case 'failed': return t.value('chat.toolFailed') || '失败'
    default: return t.value('chat.toolCall')
  }
})

const statusColor = computed(() => {
  switch (props.status) {
    case 'awaiting_confirm': return 'text-amber-500'
    case 'running': return 'text-blue-500'
    case 'success': return 'text-green-500'
    case 'failed': return 'text-red-500'
    default: return 'text-muted-foreground'
  }
})
</script>

<template>
  <div class="my-2 rounded-lg border border-border/60 bg-background/80 overflow-hidden">
    <button
      class="flex w-full items-center gap-2 px-3 py-2 text-xs font-medium text-foreground hover:bg-muted/50 transition-colors"
      @click="expanded = !expanded"
    >
      <ChevronDown v-if="expanded" class="h-3 w-3 shrink-0 text-muted-foreground" />
      <ChevronRight v-else class="h-3 w-3 shrink-0 text-muted-foreground" />
      <Wrench class="h-3.5 w-3.5 shrink-0 text-primary" />
      <span class="text-primary font-mono">{{ name }}</span>
      <span class="font-normal" :class="statusColor">{{ statusLabel }}</span>
      <ShieldAlert v-if="needsConfirm" class="h-3 w-3 text-amber-500 ml-auto" />
      <Loader2 v-else-if="status === 'running'" class="h-3 w-3 animate-spin text-blue-500 ml-auto" />
      <Check v-else-if="status === 'success'" class="h-3 w-3 text-green-500 ml-auto" />
      <X v-else-if="status === 'failed'" class="h-3 w-3 text-red-500 ml-auto" />
    </button>
    <div v-if="expanded || needsConfirm" class="border-t border-border/40 px-3 py-2 space-y-2">
      <div>
        <div class="text-[10px] uppercase tracking-wide text-muted-foreground mb-1">{{ t('chat.toolArgs') || '参数' }}</div>
        <pre class="overflow-x-auto rounded bg-muted/60 px-3 py-2 text-xs font-mono leading-relaxed text-foreground/90"><code>{{ formattedArgs }}</code></pre>
      </div>
      <div v-if="needsConfirm" class="flex items-center gap-2 pt-1">
        <button
          class="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
          @click.stop="emit('confirm')"
        >
          允许执行
        </button>
        <button
          class="rounded-md bg-muted px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted/80 transition-colors"
          @click.stop="emit('reject')"
        >
          拒绝
        </button>
      </div>
      <div v-if="resultPreview">
        <div class="text-[10px] uppercase tracking-wide text-muted-foreground mb-1">{{ t('chat.toolResult') || '结果' }}</div>
        <pre class="overflow-x-auto rounded bg-muted/60 px-3 py-2 text-xs font-mono leading-relaxed text-foreground/90 whitespace-pre-wrap"><code>{{ resultPreview }}</code></pre>
      </div>
    </div>
  </div>
</template>
