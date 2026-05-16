<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTheme } from '@/composables/useTheme'
import { useI18n } from '@/composables/useI18n'
import { useAvatar } from '@/composables/useAvatar'
import { Sun, Moon, Monitor, Upload, X } from 'lucide-vue-next'

const { theme, setTheme } = useTheme()
const { t, locale, setLocale } = useI18n()
const { userAvatar, llmAvatar, saveAvatar, removeAvatar } = useAvatar()

const themeLabels = { light: 'settings.light', dark: 'settings.dark', system: 'settings.system' } as const

// 全局默认系统提示词
const defaultSystemPrompt = ref('')

onMounted(() => {
  defaultSystemPrompt.value = localStorage.getItem('chatllm-default-system-prompt') || ''
})

function saveSystemPrompt() {
  const val = defaultSystemPrompt.value.trim()
  if (val) {
    localStorage.setItem('chatllm-default-system-prompt', val)
  } else {
    localStorage.removeItem('chatllm-default-system-prompt')
  }
}

// 头像上传处理
function handleAvatarUpload(type: 'user' | 'llm') {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/png,image/jpeg,image/webp'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = async () => {
      const dataUrl = reader.result as string
      // 提取 base64 部分(去掉 data:image/xxx;base64, 前缀)
      const base64 = dataUrl.split(',')[1]
      if (base64) {
        await saveAvatar(type, base64)
      }
    }
    reader.readAsDataURL(file)
  }
  input.click()
}
</script>

<template>
  <div class="space-y-6">
    <!-- 主题设置 -->
    <div>
      <h3 class="text-sm font-medium text-foreground mb-3">{{ t('settings.appearance') }}</h3>
      <div class="flex gap-2">
        <button
          v-for="opt in (['light', 'dark', 'system'] as const)"
          :key="opt"
          class="flex items-center gap-2 rounded-md border px-4 py-2 text-sm transition-all duration-200"
          :class="theme === opt ? 'border-primary bg-primary/10 text-foreground' : 'border-border text-muted-foreground hover:bg-muted'"
          @click="setTheme(opt)"
        >
          <Sun v-if="opt === 'light'" class="h-4 w-4" />
          <Moon v-else-if="opt === 'dark'" class="h-4 w-4" />
          <Monitor v-else class="h-4 w-4" />
          <span>{{ t(themeLabels[opt]) }}</span>
        </button>
      </div>
    </div>

    <!-- 语言设置 -->
    <div>
      <h3 class="text-sm font-medium text-foreground mb-3">{{ t('settings.language') }}</h3>
      <div class="flex gap-2">
        <button
          class="rounded-md border px-4 py-2 text-sm transition-all duration-200"
          :class="locale === 'zh' ? 'border-primary bg-primary/10 text-foreground' : 'border-border text-muted-foreground hover:bg-muted'"
          @click="setLocale('zh')"
        >
          中文
        </button>
        <button
          class="rounded-md border px-4 py-2 text-sm transition-all duration-200"
          :class="locale === 'en' ? 'border-primary bg-primary/10 text-foreground' : 'border-border text-muted-foreground hover:bg-muted'"
          @click="setLocale('en')"
        >
          English
        </button>
      </div>
    </div>

    <!-- 默认系统提示词 -->
    <div>
      <h3 class="text-sm font-medium text-foreground mb-3">{{ t('settings.systemPrompt') }}</h3>
      <textarea
        v-model="defaultSystemPrompt"
        :placeholder="t('settings.systemPromptPlaceholder')"
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 resize-y min-h-20 max-h-50 transition-all duration-200"
        rows="3"
        @blur="saveSystemPrompt"
      />
    </div>

    <!-- 头像设置 -->
    <div>
      <h3 class="text-sm font-medium text-foreground mb-3">{{ t('settings.avatar') }}</h3>
      <div class="flex gap-6">
        <!-- 用户头像 -->
        <div class="flex flex-col items-center gap-2">
          <span class="text-xs text-muted-foreground">{{ t('settings.userAvatar') }}</span>
          <div class="relative group">
            <div
              class="h-14 w-14 rounded-full overflow-hidden border-2 border-border flex items-center justify-center bg-secondary transition-all duration-200"
            >
              <img
                v-if="userAvatar"
                :src="'data:image/png;base64,' + userAvatar"
                class="h-full w-full object-cover"
              />
              <span v-else class="text-lg text-muted-foreground">U</span>
            </div>
            <button
              v-if="userAvatar"
              class="absolute -top-1 -right-1 h-5 w-5 rounded-full bg-destructive text-destructive-foreground flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200"
              @click="removeAvatar('user')"
            >
              <X class="h-3 w-3" />
            </button>
          </div>
          <button
            class="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted transition-all duration-200"
            @click="handleAvatarUpload('user')"
          >
            <Upload class="h-3 w-3" />
            {{ t('settings.uploadAvatar') }}
          </button>
        </div>

        <!-- LLM 头像 -->
        <div class="flex flex-col items-center gap-2">
          <span class="text-xs text-muted-foreground">{{ t('settings.llmAvatar') }}</span>
          <div class="relative group">
            <div
              class="h-14 w-14 rounded-full overflow-hidden border-2 border-border flex items-center justify-center bg-primary/10 transition-all duration-200"
            >
              <img
                v-if="llmAvatar"
                :src="'data:image/png;base64,' + llmAvatar"
                class="h-full w-full object-cover"
              />
              <span v-else class="text-lg text-muted-foreground">AI</span>
            </div>
            <button
              v-if="llmAvatar"
              class="absolute -top-1 -right-1 h-5 w-5 rounded-full bg-destructive text-destructive-foreground flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200"
              @click="removeAvatar('llm')"
            >
              <X class="h-3 w-3" />
            </button>
          </div>
          <button
            class="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted transition-all duration-200"
            @click="handleAvatarUpload('llm')"
          >
            <Upload class="h-3 w-3" />
            {{ t('settings.uploadAvatar') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 关于 -->
    <div>
      <h3 class="text-sm font-medium text-foreground mb-3">{{ t('settings.about') }}</h3>
      <div class="rounded-lg border border-border p-4 text-sm text-muted-foreground">
        <p>ChatLLM v0.1.0</p>
        <p class="mt-1">Desktop AI Chat Application</p>
      </div>
    </div>
  </div>
</template>
