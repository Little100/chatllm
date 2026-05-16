<script setup lang="ts">
import { Upload } from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()
const emit = defineEmits<{
  drop: [files: FileList]
  cancel: []
}>()
</script>

<template>
  <div
    class="absolute inset-0 z-50 flex items-center justify-center rounded-lg border-2 border-dashed border-primary/50 bg-primary/5 backdrop-blur-sm"
    @drop.prevent.stop="emit('drop', ($event as DragEvent).dataTransfer!.files)"
    @dragover.prevent
    @dragleave.self.prevent="emit('cancel')"
  >
    <div class="flex flex-col items-center gap-2 text-primary pointer-events-none">
      <Upload class="h-8 w-8" />
      <span class="text-sm font-medium">{{ t('dropzone.hint') }}</span>
    </div>
  </div>
</template>
