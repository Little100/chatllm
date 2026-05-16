<script setup lang="ts">
import { useTheme } from '@/composables/useTheme'
import { Sun, Moon, Monitor } from 'lucide-vue-next'

const { theme, setTheme } = useTheme()

function cycleTheme() {
  const order = ['light', 'dark', 'system'] as const
  const idx = order.indexOf(theme.value)
  setTheme(order[(idx + 1) % 3])
}
</script>

<template>
  <button
    class="inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
    @click="cycleTheme"
    :title="`Theme: ${theme}`"
  >
    <Transition name="scale-fade" mode="out-in">
      <Sun v-if="theme === 'light'" key="light" class="h-4 w-4" />
      <Moon v-else-if="theme === 'dark'" key="dark" class="h-4 w-4" />
      <Monitor v-else key="system" class="h-4 w-4" />
    </Transition>
  </button>
</template>
