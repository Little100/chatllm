<script setup lang="ts">
import { X } from 'lucide-vue-next'

interface ApiLogFull {
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

defineProps<{ log: ApiLogFull }>()
const emit = defineEmits<{ close: [] }>()

function formatJson(str: string | null): string {
  if (!str) return ''
  try {
    return JSON.stringify(JSON.parse(str), null, 2)
  } catch {
    return str
  }
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="relative w-full max-w-2xl max-h-[80vh] rounded-xl border border-border bg-background shadow-xl flex flex-col overflow-hidden">
      <div class="flex items-center justify-between border-b border-border px-6 py-4">
        <h3 class="text-sm font-semibold text-foreground">API Log Detail</h3>
        <button class="rounded-md p-1 text-muted-foreground hover:bg-muted" @click="emit('close')">
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-6 space-y-4">
        <!-- 元数据 -->
        <div class="grid grid-cols-2 gap-3 text-xs">
          <div><span class="text-muted-foreground">Model:</span> <span class="text-foreground ml-1">{{ log.model || '-' }}</span></div>
          <div><span class="text-muted-foreground">Provider:</span> <span class="text-foreground ml-1">{{ log.provider || '-' }}</span></div>
          <div><span class="text-muted-foreground">Status:</span> <span class="text-foreground ml-1">{{ log.status_code || '-' }}</span></div>
          <div><span class="text-muted-foreground">Latency:</span> <span class="text-foreground ml-1">{{ log.latency_ms ? `${log.latency_ms}ms` : '-' }}</span></div>
          <div class="col-span-2"><span class="text-muted-foreground">Time:</span> <span class="text-foreground ml-1">{{ log.created_at }}</span></div>
        </div>

        <!-- 请求体 -->
        <div>
          <h4 class="text-xs font-medium text-muted-foreground mb-1">Request Body</h4>
          <pre class="rounded-md bg-muted p-3 text-[11px] leading-relaxed overflow-x-auto max-h-48 text-foreground">{{ formatJson(log.request_body) }}</pre>
        </div>

        <!-- 响应体 -->
        <div>
          <h4 class="text-xs font-medium text-muted-foreground mb-1">Response Body</h4>
          <pre class="rounded-md bg-muted p-3 text-[11px] leading-relaxed overflow-x-auto max-h-48 text-foreground">{{ formatJson(log.response_body) }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>
