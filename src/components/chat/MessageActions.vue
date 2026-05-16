<script setup lang="ts">
import { Copy, RefreshCw, Trash2, Pencil } from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  messageId: string
  isUser: boolean
}>()

const emit = defineEmits<{
  copy: [id: string]
  regenerate: [id: string]
  delete: [id: string]
  edit: [id: string]
}>()

async function copyContent() {
  emit('copy', props.messageId)
}
</script>

<template>
  <div class="flex items-center gap-1">
    <button
      class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
      @click="copyContent"
      :title="t('chat.copy')"
    >
      <Copy class="h-3 w-3" />
    </button>
    <button
      v-if="isUser"
      class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
      @click="emit('edit', messageId)"
      :title="t('model.edit')"
    >
      <Pencil class="h-3 w-3" />
    </button>
    <button
      v-if="!isUser"
      class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
      @click="emit('regenerate', messageId)"
      :title="t('chat.regenerate')"
    >
      <RefreshCw class="h-3 w-3" />
    </button>
    <button
      class="rounded p-1 text-muted-foreground hover:bg-destructive hover:text-destructive-foreground transition-colors"
      @click="emit('delete', messageId)"
      :title="t('chat.delete')"
    >
      <Trash2 class="h-3 w-3" />
    </button>
  </div>
</template>
