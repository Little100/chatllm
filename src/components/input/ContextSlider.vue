<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  modelValue?: number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: number]
}>()

const value = ref(props.modelValue || 4096)

const displayValue = computed(() => {
  if (value.value >= 1000) return `${(value.value / 1000).toFixed(0)}K`
  return String(value.value)
})

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  value.value = Number(target.value)
  emit('update:modelValue', value.value)
}
</script>

<template>
  <div class="flex items-center gap-2">
    <span class="text-xs text-muted-foreground whitespace-nowrap">Context:</span>
    <input
      type="range"
      :value="value"
      @input="onInput"
      min="512"
      max="128000"
      step="512"
      class="h-1.5 w-24 cursor-pointer appearance-none rounded-full bg-muted accent-primary"
    />
    <span class="text-xs text-muted-foreground w-8">{{ displayValue }}</span>
  </div>
</template>
