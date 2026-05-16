<script setup lang="ts">
import { computed, ref, nextTick } from 'vue'
import MessageContent from './MessageContent.vue'
import MessageActions from './MessageActions.vue'
import { User, Bot, ChevronLeft, ChevronRight, ChevronDown } from 'lucide-vue-next'
import type { Message } from '@/types/chat'
import { useAvatar } from '@/composables/useAvatar'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  message: Message
  totalVersions: number
  currentVersion: number
}>()

const emit = defineEmits<{
  switchVersion: [parentId: string, version: number]
  editMessage: [id: string, content: string]
  deleteMessage: [id: string]
  copyMessage: [id: string, content: string]
  regenerateMessage: [id: string]
}>()

const { userAvatar, llmAvatar } = useAvatar()

const isUser = computed(() => props.message.role === 'user')
const hasVersions = computed(() => props.totalVersions > 1)
const hasError = computed(() => !!props.message.error)
/// 新格式 chat_rounds marker 下 thinking 已分散到各轮, 不再显示顶层折叠
const hasChatRoundsMarker = computed(() => props.message.content.includes('<!-- chat_rounds:'))
const hasThinking = computed(() => !!props.message.thinking && !hasChatRoundsMarker.value)
const avatarSrc = computed(() => {
  const data = isUser.value ? userAvatar.value : llmAvatar.value
  return data ? `data:image/png;base64,${data}` : null
})

const ATTACH_MARKER_RE = /\n\n<!-- user_attachments:(.*?) -->$/s
const parsedAttachments = computed<string[]>(() => {
  if (props.message.attachments?.length) return props.message.attachments
  const match = props.message.content.match(ATTACH_MARKER_RE)
  if (!match) return []
  try { return JSON.parse(match[1]) } catch { return [] }
})
const displayContent = computed(() => props.message.content.replace(ATTACH_MARKER_RE, ''))

const thinkingExpanded = ref(false)
const isEditing = ref(false)
const draft = ref('')
const editAreaRef = ref<HTMLTextAreaElement | null>(null)

function prevVersion() {
  if (props.currentVersion > 1) {
    emit('switchVersion', props.message.parent_id!, props.currentVersion - 1)
  }
}

function nextVersion() {
  if (props.currentVersion < props.totalVersions) {
    emit('switchVersion', props.message.parent_id!, props.currentVersion + 1)
  }
}

async function startEdit() {
  draft.value = props.message.content
  isEditing.value = true
  await nextTick()
  editAreaRef.value?.focus()
  if (editAreaRef.value) {
    editAreaRef.value.style.height = 'auto'
    editAreaRef.value.style.height = editAreaRef.value.scrollHeight + 'px'
  }
}

function cancelEdit() {
  isEditing.value = false
  draft.value = ''
}

function saveEdit() {
  const text = draft.value.trim()
  if (!text || text === props.message.content) {
    cancelEdit()
    return
  }
  emit('editMessage', props.message.id, text)
  isEditing.value = false
  draft.value = ''
}

function autoResize(e: Event) {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = el.scrollHeight + 'px'
}
</script>

<template>
  <div class="group flex gap-3" :class="isUser ? 'flex-row-reverse' : ''">
    <!-- 头像 -->
    <div
      class="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full text-xs font-medium"
      :class="!avatarSrc && (isUser ? 'bg-secondary text-secondary-foreground' : 'bg-primary text-primary-foreground')"
    >
      <img v-if="avatarSrc" :src="avatarSrc" class="h-full w-full object-cover" />
      <User v-else-if="isUser" class="h-3.5 w-3.5" />
      <Bot v-else class="h-3.5 w-3.5" />
    </div>

    <!-- 消息体 -->
    <div
      class="relative max-w-[80%] rounded-lg px-4 py-3 text-sm leading-relaxed"
      :class="[
        isUser ? 'bg-primary text-primary-foreground' : 'bg-muted text-foreground',
        hasError && 'border border-destructive/40'
      ]"
    >
      <template v-if="isEditing && isUser">
        <textarea
          ref="editAreaRef"
          v-model="draft"
          @input="autoResize"
          rows="2"
          class="w-full resize-none rounded-md border border-primary-foreground/30 bg-primary-foreground/10 text-primary-foreground placeholder:text-primary-foreground/50 px-2 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-primary-foreground/40"
        />
        <div class="mt-2 flex justify-end gap-2 text-xs">
          <button
            class="rounded bg-primary-foreground/15 px-2 py-1 hover:bg-primary-foreground/25 transition-colors"
            @click="cancelEdit"
          >
            {{ t('chat.cancel') }}
          </button>
          <button
            class="rounded bg-primary-foreground text-primary px-2 py-1 hover:bg-primary-foreground/90 transition-colors"
            @click="saveEdit"
          >
            {{ t('chat.save') }}
          </button>
        </div>
      </template>

      <template v-else>
        <div v-if="hasError" class="mb-2 text-xs text-destructive font-medium">
          {{ message.error }}
        </div>
        <!-- 思维链折叠 -->
        <div v-if="hasThinking" class="mb-2">
          <button
            class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
            @click="thinkingExpanded = !thinkingExpanded"
          >
            <ChevronDown v-if="thinkingExpanded" class="h-3 w-3" />
            <ChevronRight v-else class="h-3 w-3" />
            <span>{{ t('chat.thinking') }}</span>
          </button>
          <div
            v-if="thinkingExpanded"
            class="mt-1 rounded border border-border/50 bg-background/50 px-3 py-2 text-xs text-muted-foreground leading-relaxed whitespace-pre-wrap max-h-64 overflow-y-auto"
          >
            {{ message.thinking }}
          </div>
        </div>
        <!-- 附件图片 -->
        <div v-if="parsedAttachments.length > 0" class="mb-2 flex flex-wrap gap-2">
          <img
            v-for="(src, idx) in parsedAttachments"
            :key="idx"
            :src="src"
            class="max-h-48 max-w-full rounded border border-primary-foreground/20 object-contain"
          />
        </div>
        <MessageContent :content="displayContent" :is-user="isUser" />

        <!-- 版本切换 -->
        <div
          v-if="hasVersions"
          class="mt-2 flex items-center gap-1 text-xs"
          :class="isUser ? 'text-primary-foreground/70' : 'text-muted-foreground'"
        >
          <button
            class="rounded p-0.5 hover:bg-black/10 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            :disabled="currentVersion <= 1"
            @click="prevVersion"
          >
            <ChevronLeft class="h-3.5 w-3.5" />
          </button>
          <span class="min-w-8 text-center select-none">{{ currentVersion }}/{{ totalVersions }}</span>
          <button
            class="rounded p-0.5 hover:bg-black/10 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            :disabled="currentVersion >= totalVersions"
            @click="nextVersion"
          >
            <ChevronRight class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- 操作按钮 -->
        <MessageActions
          class="absolute -bottom-6 opacity-0 group-hover:opacity-100 transition-opacity"
          :class="isUser ? 'right-0' : 'left-0'"
          :message-id="message.id"
          :is-user="isUser"
          @edit="startEdit"
          @delete="emit('deleteMessage', message.id)"
          @copy="emit('copyMessage', message.id, message.content)"
          @regenerate="emit('regenerateMessage', message.id)"
        />
      </template>
    </div>
  </div>
</template>
