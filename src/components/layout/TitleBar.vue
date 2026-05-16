<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ref } from 'vue'
import { Minus, Square, X, Copy } from 'lucide-vue-next'

const appWindow = getCurrentWindow()
const isMaximized = ref(false)

async function checkMaximized() {
  isMaximized.value = await appWindow.isMaximized()
}
checkMaximized()

async function minimize() {
  await appWindow.minimize()
}

async function toggleMaximize() {
  await appWindow.toggleMaximize()
  await checkMaximized()
}

async function close() {
  await appWindow.close()
}
</script>

<template>
  <div
    class="flex h-8 items-center justify-between bg-background border-b border-border select-none"
    data-tauri-drag-region
  >
    <div class="flex items-center gap-2 px-3" data-tauri-drag-region>
      <span class="text-xs font-medium text-foreground" data-tauri-drag-region>ChatLLM</span>
    </div>
    <div class="flex items-center">
      <button
        class="inline-flex h-8 w-10 items-center justify-center text-muted-foreground hover:bg-muted transition-colors"
        @click="minimize"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="inline-flex h-8 w-10 items-center justify-center text-muted-foreground hover:bg-muted transition-colors"
        @click="toggleMaximize"
      >
        <Square v-if="!isMaximized" class="h-3 w-3" />
        <Copy v-else class="h-3 w-3" />
      </button>
      <button
        class="inline-flex h-8 w-10 items-center justify-center text-muted-foreground hover:bg-destructive hover:text-destructive-foreground transition-colors"
        @click="close"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </div>
</template>
