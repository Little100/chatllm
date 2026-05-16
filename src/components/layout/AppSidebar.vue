<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Plus, Search, Settings, MessageSquare, Trash2, Pencil, Loader2 } from 'lucide-vue-next'
import ThemeToggle from './ThemeToggle.vue'
import { useI18n } from '@/composables/useI18n'
import { useStreamStore } from '@/stores/streamStore'

const { t } = useI18n()
const streamStore = useStreamStore()

interface SessionItem {
  id: string
  title: string
  updated_at: string
  pinned: boolean
}

const sessions = ref<SessionItem[]>([])
const activeSessionId = ref<string | null>(null)
const searchQuery = ref('')
const editingId = ref<string | null>(null)
const editTitle = ref('')
const errorMsg = ref('')

const emit = defineEmits<{
  selectSession: [id: string | null]
  newChat: []
  openSettings: []
}>()

const filteredSessions = computed(() => {
  if (!searchQuery.value) return sessions.value
  const q = searchQuery.value.toLowerCase()
  return sessions.value.filter((s) => s.title.toLowerCase().includes(q))
})

function showError(key: string) {
  errorMsg.value = t.value(key)
  setTimeout(() => { errorMsg.value = '' }, 3000)
}

async function loadSessions() {
  try {
    sessions.value = await invoke<SessionItem[]>('list_sessions')
  } catch (e) {
    console.error(e)
    showError('error.loadFailed')
  }
}

async function createSession() {
  try {
    const session = await invoke<SessionItem>('create_session', {
      title: t.value('sidebar.newChat'),
      modelConfigId: null,
    })
    sessions.value.unshift(session)
    activeSessionId.value = session.id
    emit('selectSession', session.id)
  } catch (e) {
    console.error(e)
    showError('error.createFailed')
  }
}

async function deleteSession(id: string) {
  try {
    await invoke('delete_session', { id })
    sessions.value = sessions.value.filter((s) => s.id !== id)
    if (activeSessionId.value === id) {
      activeSessionId.value = sessions.value[0]?.id || null
      emit('selectSession', activeSessionId.value)
    }
  } catch (e) {
    console.error(e)
    showError('error.deleteFailed')
  }
}

function startRename(session: SessionItem) {
  editingId.value = session.id
  editTitle.value = session.title
}

async function finishRename(id: string) {
  if (editTitle.value.trim()) {
    try {
      await invoke('rename_session', { id, title: editTitle.value.trim() })
      const s = sessions.value.find((s) => s.id === id)
      if (s) s.title = editTitle.value.trim()
    } catch (e) {
      console.error(e)
      showError('error.renameFailed')
    }
  }
  editingId.value = null
}

function selectSession(id: string) {
  activeSessionId.value = id
  emit('selectSession', id)
}

loadSessions()

// 后台流完成时刷新列表(updated_at 可能变化)
const unlistenFinished = listen('stream-finished', () => { loadSessions() })
const unlistenInterrupted = listen('stream-interrupted', () => { loadSessions() })
onUnmounted(() => {
  unlistenFinished.then(fn => fn())
  unlistenInterrupted.then(fn => fn())
})

defineExpose({ loadSessions, activeSessionId })
</script>

<template>
  <aside class="flex h-full w-64 flex-col border-r border-border bg-sidebar-background">
    <!-- 顶部操作区 -->
    <div class="flex items-center gap-2 p-3 border-b border-sidebar-border">
      <button
        class="flex-1 flex items-center gap-2 rounded-md bg-sidebar-accent px-3 py-1.5 text-sm text-sidebar-accent-foreground hover:bg-sidebar-primary hover:text-sidebar-primary-foreground transition-all duration-200"
        @click="createSession"
      >
        <Plus class="h-4 w-4" />
        <span>{{ t('sidebar.newChat') }}</span>
      </button>
    </div>

    <!-- 搜索框 -->
    <div class="px-3 py-2">
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('sidebar.search')"
          class="w-full rounded-md border border-sidebar-border bg-transparent py-1.5 pl-8 pr-3 text-xs text-sidebar-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-sidebar-ring transition-shadow duration-200"
        />
      </div>
    </div>

    <!-- 会话列表 -->
    <div class="flex-1 overflow-y-auto px-2 py-1">
      <TransitionGroup name="list" tag="div" class="relative">
        <div
          v-for="session in filteredSessions"
          :key="session.id"
          class="group relative mb-0.5 flex items-center rounded-md px-2 py-2 text-sm cursor-pointer"
          :class="activeSessionId === session.id ? 'bg-sidebar-accent text-sidebar-accent-foreground' : 'text-sidebar-foreground hover:bg-sidebar-accent/50'"
          @click="selectSession(session.id)"
        >
          <MessageSquare class="mr-2 h-3.5 w-3.5 shrink-0 opacity-60" />
          <Loader2 v-if="streamStore.isActive(session.id)" class="mr-1 h-3 w-3 shrink-0 animate-spin text-primary" />
          <template v-if="editingId === session.id">
            <input
              v-model="editTitle"
              class="flex-1 bg-transparent text-xs outline-none border-b border-sidebar-ring"
              @keydown.enter="finishRename(session.id)"
              @blur="finishRename(session.id)"
              @click.stop
              autofocus
            />
          </template>
          <template v-else>
            <span class="flex-1 truncate text-xs">{{ session.title }}</span>
          </template>
          <div class="hidden group-hover:flex items-center gap-0.5 ml-1 animate-in fade-in duration-150">
            <button
              class="rounded p-0.5 hover:bg-sidebar-border"
              @click.stop="startRename(session)"
            >
              <Pencil class="h-3 w-3" />
            </button>
            <button
              class="rounded p-0.5 hover:bg-destructive hover:text-destructive-foreground"
              @click.stop="deleteSession(session.id)"
            >
              <Trash2 class="h-3 w-3" />
            </button>
          </div>
        </div>
      </TransitionGroup>
      <Transition name="fade">
        <div v-if="filteredSessions.length === 0" class="px-3 py-6 text-center text-xs text-muted-foreground">
          {{ t('sidebar.noSessions') }}
        </div>
      </Transition>
    </div>

    <!-- 底部操作 -->
    <div class="flex items-center justify-between border-t border-sidebar-border px-3 py-2">
      <ThemeToggle />
      <button
        class="inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-all duration-200"
        @click="emit('openSettings')"
      >
        <Settings class="h-4 w-4" />
      </button>
    </div>
    <div v-if="errorMsg" class="text-red-500 text-xs px-3 py-1">{{ errorMsg }}</div>
  </aside>
</template>
