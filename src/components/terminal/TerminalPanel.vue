<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { useTheme } from '@/composables/useTheme'

const props = defineProps<{ terminalId: string }>()

const containerRef = ref<HTMLElement | null>(null)
const { isDark } = useTheme()

const darkTheme = { background: '#0a0a0a', foreground: '#fafafa' }
const lightTheme = { background: '#ffffff', foreground: '#171717' }

let term: Terminal | null = null
let fitAddon: FitAddon | null = null
let unlisten: (() => void) | null = null
let resizeObserver: ResizeObserver | null = null
let rafId = 0
let lastCols = 0
let lastRows = 0

onMounted(async () => {
  if (!containerRef.value) return

  term = new Terminal({
    fontSize: 13,
    fontFamily: "'JetBrains Mono', 'Cascadia Code', monospace",
    cursorBlink: true,
    theme: isDark.value ? darkTheme : lightTheme,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(containerRef.value)
  fitAddon.fit()
  lastCols = term.cols
  lastRows = term.rows

  // 输入转发
  term.onData((data) => {
    invoke('write_to_terminal', { id: props.terminalId, data })
  })

  // 监听输出(base64 解码后写入终端)
  unlisten = await listen<string>(`terminal-output:${props.terminalId}`, (event) => {
    const binary = atob(event.payload)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i)
    }
    term?.write(bytes)
  }) as unknown as () => void

  // 尺寸变化合并到下一帧, 仅在 cols/rows 实际变化时同步给后端
  resizeObserver = new ResizeObserver(() => {
    if (rafId) return
    rafId = requestAnimationFrame(() => {
      rafId = 0
      fitAddon?.fit()
      if (!term) return
      const { cols, rows } = term
      if (cols !== lastCols || rows !== lastRows) {
        lastCols = cols
        lastRows = rows
        invoke('resize_terminal', {
          id: props.terminalId,
          cols,
          rows,
        })
      }
    })
  })
  resizeObserver.observe(containerRef.value)
})

// 跟随全局主题切换
watch(isDark, (dark) => {
  if (term) {
    term.options.theme = dark ? darkTheme : lightTheme
  }
})

onUnmounted(() => {
  if (rafId) {
    cancelAnimationFrame(rafId)
    rafId = 0
  }
  unlisten?.()
  resizeObserver?.disconnect()
  term?.dispose()
})
</script>

<template>
  <div ref="containerRef" class="h-full w-full" />
</template>
