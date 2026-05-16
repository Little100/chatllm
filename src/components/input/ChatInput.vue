<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from 'vue'
import { Send, Paperclip, Square } from 'lucide-vue-next'
import AttachmentPreview from './AttachmentPreview.vue'
import FileDropZone from './FileDropZone.vue'
import { useFileUpload } from '@/composables/useFileUpload'
import { useI18n } from '@/composables/useI18n'
import { useModelSelection } from '@/composables/useModelSelection'

const { t } = useI18n()
const { modelConfigId, loadConfigs } = useModelSelection()

export interface SendPayload {
  text: string
  attachments: string[]
  modelConfigId: string
}

const props = defineProps<{
  isStreaming: boolean
  disabled: boolean
}>()

const emit = defineEmits<{
  send: [payload: SendPayload]
  cancel: []
}>()

const textareaRef = ref<HTMLTextAreaElement | null>(null)
const content = ref('')
const dragCounter = ref(0)
const isDragging = computed(() => dragCounter.value > 0)
const { files, addFiles, addFilesFromPaths, removeFile, clearFiles } = useFileUpload()

onMounted(async () => {
  await loadConfigs()
  nextTick(autoResize)
})

const canSend = computed(() => (content.value.trim().length > 0 || files.value.length > 0) && !props.disabled)

function autoResize() {
  const el = textareaRef.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.max(36, Math.min(el.scrollHeight, 200)) + 'px'
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

function handleSend() {
  if (!canSend.value) return
  const imageFiles = files.value.filter(f => f.base64 && f.type.startsWith('image/'))
  const textFiles = files.value.filter(f => f.base64 && !f.type.startsWith('image/'))
  const pathOnlyFiles = files.value.filter(f => !f.base64 && f.filePath)

  const attachments = imageFiles.map(f => `data:${f.type};base64,${f.base64}`)

  let text = content.value.trim()
  for (const f of textFiles) {
    const bytes = Uint8Array.from(atob(f.base64!), c => c.charCodeAt(0))
    const decoded = new TextDecoder('utf-8').decode(bytes)
    const header = f.filePath ? `[${f.filePath}]` : `[${f.name}]`
    text += `\n\n${header}\n${decoded}`
  }
  for (const f of pathOnlyFiles) {
    text += `\n\n[file_path: ${f.filePath}]`
  }

  emit('send', {
    text,
    attachments,
    modelConfigId: modelConfigId.value,
  })
  content.value = ''
  clearFiles()
  nextTick(autoResize)
}

async function handlePaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items
  if (!items) return
  const imageFiles: File[] = []
  for (const item of Array.from(items)) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile()
      if (file) imageFiles.push(file)
    }
  }
  if (imageFiles.length > 0) {
    await addFiles(imageFiles)
  }
}

function handleDrop(fileList: FileList) {
  addFiles(fileList)
  dragCounter.value = 0
}

function handleDragCancel() {
  dragCounter.value = 0
}

defineExpose({ addFiles, addFilesFromPaths })

function triggerFileInput() {
  const input = document.createElement('input')
  input.type = 'file'
  input.multiple = true
  input.accept = 'image/*,.pdf,.txt,.md,.json,.csv'
  input.onchange = () => {
    if (input.files) addFiles(input.files)
  }
  input.click()
}
</script>

<template>
  <div
    class="relative border-t border-border bg-background px-4 py-3"
    @dragenter.prevent="dragCounter++"
    @dragover.prevent
    @dragleave.prevent="dragCounter--"
    @drop.prevent="handleDrop($event.dataTransfer!.files)"
  >
    <FileDropZone v-if="isDragging" @drop="handleDrop" @cancel="handleDragCancel" />

    <AttachmentPreview
      v-if="files.length > 0"
      :files="files"
      @remove="removeFile"
      class="mb-2"
    />

    <div class="mx-auto flex max-w-3xl items-center gap-2">
      <button
        class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-colors disabled:opacity-50"
        @click="triggerFileInput"
        :disabled="disabled"
      >
        <Paperclip class="h-4 w-4" />
      </button>

      <div class="relative flex-1">
        <textarea
          ref="textareaRef"
          v-model="content"
          @input="autoResize"
          @keydown="handleKeydown"
          @paste="handlePaste"
          :disabled="disabled"
          :placeholder="t('chat.placeholder')"
          rows="1"
          class="block w-full resize-none rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm leading-6 placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 transition-shadow duration-200"
          style="min-height: 40px; max-height: 200px"
        />
      </div>

      <button
        v-if="isStreaming"
        class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-colors"
        @click="emit('cancel')"
      >
        <Square class="h-4 w-4" />
      </button>
      <button
        v-else
        class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
        :disabled="!canSend"
        @click="handleSend"
      >
        <Send class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>
