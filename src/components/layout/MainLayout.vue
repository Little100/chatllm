<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ScrollText, ChevronDown, ChevronUp } from 'lucide-vue-next'
import TitleBar from './TitleBar.vue'
import AppSidebar from './AppSidebar.vue'
import ChatView from '@/components/chat/ChatView.vue'
import SettingsDialog from '@/components/settings/SettingsDialog.vue'
import ApiLogViewer from '@/components/logs/ApiLogViewer.vue'
import { useModelSelection } from '@/composables/useModelSelection'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()
const {
  modelConfigs,
  modelConfigId,
  localReasoning,
  localTemperature,
  localTemperatureEnabled,
  localTopP,
  localTopPEnabled,
  localMaxTokens,
  debouncedSave,
  loadConfigs,
} = useModelSelection()

const activeSessionId = ref<string | null>(null)
const showSettings = ref(false)
const activeView = ref<'chat' | 'logs'>('chat')
const showAdvanced = ref(false)
const sidebarRef = ref<InstanceType<typeof AppSidebar> | null>(null)

onMounted(() => loadConfigs())

function onSelectSession(id: string | null) {
  activeSessionId.value = id
  activeView.value = 'chat'
}

function onNewChat() {
  activeSessionId.value = null
  activeView.value = 'chat'
}

function onSessionCreated(id: string) {
  activeSessionId.value = id
  activeView.value = 'chat'
  sidebarRef.value?.loadSessions()
  if (sidebarRef.value) {
    sidebarRef.value.activeSessionId = id
  }
}
</script>

<template>
  <div class="flex h-screen w-screen flex-col overflow-hidden bg-background">
    <TitleBar />
    <div class="flex flex-1 overflow-hidden">
      <AppSidebar
        ref="sidebarRef"
        @select-session="onSelectSession"
        @new-chat="onNewChat"
        @open-settings="showSettings = true"
      />
      <div class="flex flex-1 flex-col overflow-hidden">
        <main class="flex-1 overflow-hidden">
          <Transition name="fade" mode="out-in">
            <ChatView v-if="activeView === 'chat'" key="chat" :session-id="activeSessionId" @session-created="onSessionCreated" />
            <ApiLogViewer v-else-if="activeView === 'logs'" key="logs" />
          </Transition>
        </main>

        <!-- 底部工具栏 -->
        <div class="flex items-center gap-1 border-t border-border bg-muted/30 px-3 py-1">
          <button
            class="rounded p-1"
            :class="activeView === 'logs' ? 'text-foreground bg-muted' : 'text-muted-foreground hover:text-foreground hover:bg-muted'"
            title="API Logs"
            @click="activeView = activeView === 'logs' ? 'chat' : 'logs'"
          >
            <ScrollText class="h-4 w-4" />
          </button>

          <div class="ml-auto flex items-center gap-2">
            <!-- 模型选择 -->
            <select
              v-model="modelConfigId"
              class="rounded border border-input bg-background text-foreground px-2 py-0.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring max-w-35 truncate"
            >
              <option v-for="cfg in modelConfigs" :key="cfg.id" :value="cfg.id">
                {{ cfg.name }}
              </option>
            </select>

            <!-- 推理强度 -->
            <div class="flex items-center gap-1">
              <span class="text-[10px] text-muted-foreground">{{ t('model.reasoningEffort') }}</span>
              <select
                v-model="localReasoning"
                @change="debouncedSave()"
                class="rounded border border-input bg-background text-foreground px-1.5 py-0.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="none">none</option>
                <option value="minimal">minimal</option>
                <option value="low">low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
                <option value="xhigh">xhigh</option>
                <option value="max">max</option>
              </select>
            </div>

            <!-- 展开高级参数 -->
            <button
              class="flex items-center gap-0.5 rounded p-1 text-[10px] text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
              @click="showAdvanced = !showAdvanced"
            >
              <component :is="showAdvanced ? ChevronUp : ChevronDown" class="h-3 w-3" />
              <span>{{ showAdvanced ? t('model.hideAdvanced') : t('model.showAdvanced') }}</span>
            </button>
          </div>
        </div>

        <!-- 高级参数行 -->
        <div v-if="showAdvanced" class="flex items-center gap-3 border-t border-border bg-muted/20 px-3 py-1">
          <!-- Temperature -->
          <div class="flex items-center gap-1">
            <button
              class="relative inline-flex h-3 w-5 items-center rounded-full transition-colors duration-200 shrink-0"
              :class="localTemperatureEnabled ? 'bg-primary' : 'bg-muted-foreground/30'"
              @click="localTemperatureEnabled = !localTemperatureEnabled; debouncedSave()"
            >
              <span
                class="inline-block h-2 w-2 rounded-full bg-white transition-transform duration-200"
                :class="localTemperatureEnabled ? 'translate-x-2.5' : 'translate-x-0.5'"
              />
            </button>
            <span class="text-[10px] text-muted-foreground">T</span>
            <input
              v-model.number="localTemperature"
              @input="debouncedSave()"
              type="number"
              step="0.1"
              min="0"
              max="2"
              :disabled="!localTemperatureEnabled"
              class="w-14 rounded border border-input bg-background text-foreground px-1.5 py-0.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-40"
            />
          </div>

          <!-- Top P -->
          <div class="flex items-center gap-1">
            <button
              class="relative inline-flex h-3 w-5 items-center rounded-full transition-colors duration-200 shrink-0"
              :class="localTopPEnabled ? 'bg-primary' : 'bg-muted-foreground/30'"
              @click="localTopPEnabled = !localTopPEnabled; debouncedSave()"
            >
              <span
                class="inline-block h-2 w-2 rounded-full bg-white transition-transform duration-200"
                :class="localTopPEnabled ? 'translate-x-2.5' : 'translate-x-0.5'"
              />
            </button>
            <span class="text-[10px] text-muted-foreground">P</span>
            <input
              v-model.number="localTopP"
              @input="debouncedSave()"
              type="number"
              step="0.1"
              min="0"
              max="1"
              :disabled="!localTopPEnabled"
              class="w-14 rounded border border-input bg-background text-foreground px-1.5 py-0.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-40"
            />
          </div>

          <!-- Max Tokens -->
          <div class="flex items-center gap-1">
            <span class="text-[10px] text-muted-foreground">MaxTk</span>
            <input
              v-model.number="localMaxTokens"
              @input="debouncedSave()"
              type="number"
              min="1"
              step="256"
              class="w-20 rounded border border-input bg-background text-foreground px-1.5 py-0.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            />
          </div>
        </div>
      </div>
    </div>
    <Transition name="fade">
      <SettingsDialog v-if="showSettings" @close="showSettings = false" />
    </Transition>
  </div>
</template>
