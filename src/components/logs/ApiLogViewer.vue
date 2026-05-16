<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ApiLogDetail from './ApiLogDetail.vue'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

interface ApiLog {
  id: string
  session_id: string | null
  model: string | null
  provider: string | null
  status_code: number | null
  latency_ms: number | null
  created_at: string
}

interface ApiLogFull extends ApiLog {
  request_body: string | null
  response_body: string | null
}

const logs = ref<ApiLog[]>([])
const selectedLog = ref<ApiLogFull | null>(null)
const modelFilter = ref('')
const errorMsg = ref('')

function showError(key: string) {
  errorMsg.value = t.value(key)
  setTimeout(() => { errorMsg.value = '' }, 3000)
}

async function loadLogs() {
  try {
    const params: Record<string, unknown> = { limit: 100, offset: 0 }
    if (modelFilter.value) params.modelFilter = modelFilter.value
    logs.value = await invoke<ApiLog[]>('list_logs', params)
  } catch (e) {
    console.error(e)
    showError('error.loadFailed')
  }
}

async function viewDetail(id: string) {
  try {
    selectedLog.value = await invoke<ApiLogFull>('get_log_detail', { id })
  } catch (e) {
    console.error(e)
    showError('error.loadFailed')
  }
}

function formatTime(iso: string) {
  return new Date(iso).toLocaleString()
}

onMounted(loadLogs)
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- 过滤栏 -->
    <div class="flex items-center gap-3 border-b border-border px-4 py-3">
      <input
        v-model="modelFilter"
        placeholder="Filter by model..."
        class="rounded-md border border-input bg-background text-foreground px-3 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
        @keydown.enter="loadLogs"
      />
      <button
        class="rounded-md bg-secondary px-3 py-1.5 text-xs text-secondary-foreground hover:bg-secondary/80 transition-colors"
        @click="loadLogs"
      >
        Refresh
      </button>
      <span v-if="errorMsg" class="text-red-500 text-xs">{{ errorMsg }}</span>
    </div>

    <!-- 日志列表 -->
    <div class="flex-1 overflow-y-auto">
      <table class="w-full text-xs">
        <thead class="sticky top-0 bg-muted">
          <tr class="border-b border-border">
            <th class="px-3 py-2 text-left font-medium text-muted-foreground">Time</th>
            <th class="px-3 py-2 text-left font-medium text-muted-foreground">Model</th>
            <th class="px-3 py-2 text-left font-medium text-muted-foreground">Status</th>
            <th class="px-3 py-2 text-left font-medium text-muted-foreground">Latency</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="log in logs"
            :key="log.id"
            class="border-b border-border/50 cursor-pointer hover:bg-muted/50 transition-colors"
            @click="viewDetail(log.id)"
          >
            <td class="px-3 py-2 text-muted-foreground">{{ formatTime(log.created_at) }}</td>
            <td class="px-3 py-2 text-foreground">{{ log.model || '-' }}</td>
            <td class="px-3 py-2">
              <span
                class="inline-flex rounded-full px-1.5 py-0.5 text-[10px] font-medium"
                :class="log.status_code && log.status_code < 400 ? 'bg-green-500/10 text-green-600' : 'bg-destructive/10 text-destructive'"
              >
                {{ log.status_code || '-' }}
              </span>
            </td>
            <td class="px-3 py-2 text-muted-foreground">{{ log.latency_ms ? `${log.latency_ms}ms` : '-' }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="logs.length === 0" class="py-12 text-center text-sm text-muted-foreground">
        No API logs yet
      </div>
    </div>

    <!-- 详情弹窗 -->
    <ApiLogDetail v-if="selectedLog" :log="selectedLog" @close="selectedLog = null" />
  </div>
</template>
