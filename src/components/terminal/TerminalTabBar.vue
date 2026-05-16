<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus, X } from 'lucide-vue-next'
import TerminalPanel from './TerminalPanel.vue'

interface TerminalTab {
  id: string
  title: string
}

const tabs = ref<TerminalTab[]>([])
const activeTabId = ref<string | null>(null)

async function createTab() {
  const id = crypto.randomUUID()
  try {
    await invoke('create_terminal', { id, shell: null })
    tabs.value.push({ id, title: `Terminal ${tabs.value.length + 1}` })
    activeTabId.value = id
  } catch (e) {
    console.error(e)
  }
}

async function closeTab(id: string) {
  try {
    await invoke('close_terminal', { id })
  } catch (_) {}
  tabs.value = tabs.value.filter((t) => t.id !== id)
  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[0]?.id || null
  }
}
</script>

<template>
  <div class="flex h-full flex-col border-t border-border bg-background">
    <!-- Tab 栏 -->
    <div class="flex items-center border-b border-border bg-muted/50 px-2">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="group flex items-center gap-1.5 border-r border-border px-3 py-1.5 text-xs cursor-pointer transition-colors"
        :class="activeTabId === tab.id ? 'bg-background text-foreground' : 'text-muted-foreground hover:text-foreground'"
        @click="activeTabId = tab.id"
      >
        <span>{{ tab.title }}</span>
        <button
          class="rounded p-0.5 opacity-0 group-hover:opacity-100 hover:bg-destructive hover:text-destructive-foreground transition-all"
          @click.stop="closeTab(tab.id)"
        >
          <X class="h-3 w-3" />
        </button>
      </div>
      <button
        class="ml-1 rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
        @click="createTab"
      >
        <Plus class="h-3.5 w-3.5" />
      </button>
    </div>

    <!-- 终端内容 -->
    <div class="flex-1 overflow-hidden">
      <TerminalPanel
        v-if="activeTabId"
        :key="activeTabId"
        :terminal-id="activeTabId"
      />
      <div v-else class="flex h-full items-center justify-center text-sm text-muted-foreground">
        Click + to open a terminal
      </div>
    </div>
  </div>
</template>
