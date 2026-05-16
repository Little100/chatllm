<script setup lang="ts">
import { ref } from 'vue'
import { Eye, EyeOff } from 'lucide-vue-next'

defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const visible = ref(false)

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="relative">
    <input
      :type="visible ? 'text' : 'password'"
      :value="modelValue"
      @input="onInput"
      :placeholder="placeholder"
      class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 pr-9 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
    />
    <button
      type="button"
      class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
      @click="visible = !visible"
    >
      <EyeOff v-if="visible" class="h-3.5 w-3.5" />
      <Eye v-else class="h-3.5 w-3.5" />
    </button>
  </div>
</template>
