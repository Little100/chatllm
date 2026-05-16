<script setup lang="ts">
import { X, FileText, MapPin } from 'lucide-vue-next'
import type { UploadFile } from '@/composables/useFileUpload'

defineProps<{ files: UploadFile[] }>()
const emit = defineEmits<{ remove: [id: string] }>()
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <div
      v-for="file in files"
      :key="file.id"
      class="group relative flex items-center gap-2 rounded-md border border-border bg-muted px-2 py-1.5"
    >
      <img
        v-if="file.previewUrl"
        :src="file.previewUrl"
        class="h-8 w-8 rounded object-cover"
      />
      <div v-else class="flex h-8 w-8 items-center justify-center rounded bg-secondary">
        <FileText class="h-4 w-4 text-muted-foreground" />
      </div>
      <span class="max-w-[120px] truncate text-xs text-foreground">{{ file.name }}</span>
      <MapPin v-if="file.filePath" class="h-3 w-3 text-muted-foreground" />
      <button
        class="ml-1 rounded-full p-0.5 text-muted-foreground hover:bg-destructive hover:text-destructive-foreground transition-colors"
        @click="emit('remove', file.id)"
      >
        <X class="h-3 w-3" />
      </button>
    </div>
  </div>
</template>
