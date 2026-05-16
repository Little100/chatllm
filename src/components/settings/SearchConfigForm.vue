<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus, Trash2, RefreshCw } from 'lucide-vue-next'
import ApiKeyField from './ApiKeyField.vue'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

interface SearchConfig {
  id: string
  name: string
  model: string
  base_url: string | null
  api_key_name: string | null
  api_key: string | null
  max_tokens: number
  context_window: number
  temperature: number
  top_p: number
  temperature_enabled: boolean
  top_p_enabled: boolean
  reasoning_effort: string | null
  prompt_template: string
  enabled: boolean
  created_at: string
}

const searchConfigs = ref<SearchConfig[]>([])
const showForm = ref(false)
const editingConfig = ref<SearchConfig | null>(null)

const form = ref({
  name: '',
  model: '',
  base_url: '',
  api_key_name: '',
  max_tokens: 4096,
  context_window: 128000,
  temperature: 0.7,
  top_p: 1.0,
  temperature_enabled: true,
  top_p_enabled: true,
  reasoning_effort: 'none',
  prompt_template: '请搜索以下内容并总结: {query}',
  enabled: true,
})

const apiKey = ref('')
const modelList = ref<string[]>([])
const fetching = ref(false)
const fetchError = ref('')
const saveError = ref('')

onMounted(async () => {
  await loadData()
})

async function loadData() {
  try {
    searchConfigs.value = await invoke<SearchConfig[]>('list_search_configs')
  } catch (_) {}
}

async function openForm(config?: SearchConfig) {
  fetchError.value = ''
  saveError.value = ''
  modelList.value = []
  apiKey.value = ''
  if (config) {
    editingConfig.value = config
    form.value = {
      name: config.name,
      model: config.model,
      base_url: config.base_url || '',
      api_key_name: config.api_key_name || '',
      max_tokens: config.max_tokens,
      context_window: config.context_window ?? 128000,
      temperature: config.temperature,
      top_p: config.top_p,
      temperature_enabled: config.temperature_enabled ?? true,
      top_p_enabled: config.top_p_enabled ?? true,
      reasoning_effort: config.reasoning_effort || 'none',
      prompt_template: config.prompt_template,
      enabled: config.enabled,
    }
    if (config.api_key) {
      apiKey.value = config.api_key
    } else if (config.api_key_name) {
      try {
        apiKey.value = await invoke<string>('get_api_key', { service: config.api_key_name })
      } catch (_) {}
    }
  } else {
    editingConfig.value = null
    form.value = {
      name: '',
      model: '',
      base_url: '',
      api_key_name: '',
      max_tokens: 4096,
      context_window: 128000,
      temperature: 0.7,
      top_p: 1.0,
      temperature_enabled: true,
      top_p_enabled: true,
      reasoning_effort: 'none',
      prompt_template: '请搜索以下内容并总结: {query}',
      enabled: true,
    }
  }
  showForm.value = true
}

function closeForm() {
  showForm.value = false
  editingConfig.value = null
}

async function fetchModels() {
  if (!form.value.base_url || !apiKey.value) {
    fetchError.value = '请先填写接口地址和 API Key'
    return
  }
  fetching.value = true
  fetchError.value = ''
  try {
    modelList.value = await invoke<string[]>('fetch_model_list', {
      baseUrl: form.value.base_url,
      apiKey: apiKey.value,
    })
  } catch (e: any) {
    fetchError.value = String(e)
  } finally {
    fetching.value = false
  }
}

async function handleSave() {
  saveError.value = ''
  const keyName = form.value.api_key_name || `search_${Date.now()}`

  const params = {
    name: form.value.name,
    model: form.value.model,
    baseUrl: form.value.base_url || null,
    apiKeyName: keyName,
    apiKey: apiKey.value || null,
    maxTokens: form.value.max_tokens,
    contextWindow: form.value.context_window,
    temperature: form.value.temperature,
    topP: form.value.top_p,
    temperatureEnabled: form.value.temperature_enabled,
    topPEnabled: form.value.top_p_enabled,
    reasoningEffort: form.value.reasoning_effort === 'none' ? null : form.value.reasoning_effort,
    promptTemplate: form.value.prompt_template,
    enabled: form.value.enabled,
  }

  try {
    if (editingConfig.value) {
      await invoke('update_search_config', { id: editingConfig.value.id, params })
    } else {
      await invoke('create_search_config', { params })
    }
    closeForm()
    await loadData()
  } catch (e: any) {
    console.error(e)
    saveError.value = String(e)
  }
}

async function handleDelete(id: string) {
  try {
    await invoke('delete_search_config', { id })
    await loadData()
  } catch (e) {
    console.error(e)
  }
}
</script>

<template>
  <div class="space-y-4">
    <template v-if="!showForm">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium text-foreground">{{ t('search.config') }}</h3>
        <button
          class="flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:bg-primary/90 transition-colors"
          @click="openForm()"
        >
          <Plus class="h-3.5 w-3.5" />
          {{ t('model.add') }}
        </button>
      </div>

      <div v-if="searchConfigs.length === 0" class="text-center py-8 text-sm text-muted-foreground">
        {{ t('search.noConfig') }}
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="config in searchConfigs"
          :key="config.id"
          class="flex items-center justify-between rounded-lg border border-border bg-background p-3 hover:bg-muted/50 transition-colors cursor-pointer"
          @click="openForm(config)"
        >
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-foreground truncate">{{ config.name }}</span>
              <span
                class="inline-flex items-center rounded-full px-2 py-0.5 text-xs"
                :class="config.enabled ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-muted text-muted-foreground'"
              >
                {{ config.enabled ? t('search.enabled') : t('search.disabled') }}
              </span>
            </div>
            <p class="text-xs text-muted-foreground mt-0.5 truncate">{{ config.model }}</p>
          </div>
          <button
            class="ml-2 rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors"
            @click.stop="handleDelete(config.id)"
          >
            <Trash2 class="h-4 w-4" />
          </button>
        </div>
      </div>
    </template>

    <template v-else>
      <h3 class="text-sm font-medium text-foreground">
        {{ editingConfig ? t('model.edit') : t('model.add') }}
      </h3>

      <div class="grid gap-3">
        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.name') }}</label>
          <input
            v-model="form.name"
            class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
            placeholder="联网搜索"
          />
        </div>

        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.modelId') }}</label>
          <div class="flex gap-1">
            <input
              v-model="form.model"
              list="search-model-list-options"
              class="flex-1 rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
              placeholder="gpt-4o"
            />
            <datalist id="search-model-list-options">
              <option v-for="m in modelList" :key="m" :value="m" />
            </datalist>
            <button
              type="button"
              class="shrink-0 rounded-md border border-input bg-background text-foreground px-2 hover:bg-muted transition-colors duration-200 disabled:opacity-50"
              :disabled="fetching"
              :title="t('model.fetchModels')"
              @click="fetchModels"
            >
              <RefreshCw class="size-4" :class="{ 'animate-spin': fetching }" />
            </button>
          </div>
        </div>

        <p v-if="fetchError" class="text-xs text-destructive">{{ fetchError }}</p>

        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.baseUrl') }}</label>
          <input
            v-model="form.base_url"
            class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
            placeholder="https://api.openai.com"
          />
        </div>

        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.apiKey') }}</label>
          <ApiKeyField v-model="apiKey" placeholder="sk-..." />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.contextWindow') }}</label>
            <input v-model.number="form.context_window" type="number" min="0" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200" />
          </div>
          <div>
            <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.maxTokens') }}</label>
            <input v-model.number="form.max_tokens" type="number" min="0" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200" />
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <div class="flex items-center justify-between mb-1">
              <label class="text-xs text-muted-foreground">{{ t('model.temperature') }}</label>
              <button
                type="button"
                class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200"
                :class="form.temperature_enabled ? 'bg-primary' : 'bg-muted-foreground/30'"
                :title="t('model.enableTemperature')"
                @click="form.temperature_enabled = !form.temperature_enabled"
              >
                <span
                  class="inline-block h-2.5 w-2.5 rounded-full bg-white transition-transform duration-200"
                  :class="form.temperature_enabled ? 'translate-x-3.5' : 'translate-x-0.75'"
                />
              </button>
            </div>
            <input
              v-model.number="form.temperature"
              type="number"
              step="0.1"
              min="0"
              max="2"
              :disabled="!form.temperature_enabled"
              class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200 disabled:opacity-50"
            />
          </div>
          <div>
            <div class="flex items-center justify-between mb-1">
              <label class="text-xs text-muted-foreground">{{ t('model.topP') }}</label>
              <button
                type="button"
                class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200"
                :class="form.top_p_enabled ? 'bg-primary' : 'bg-muted-foreground/30'"
                :title="t('model.enableTopP')"
                @click="form.top_p_enabled = !form.top_p_enabled"
              >
                <span
                  class="inline-block h-2.5 w-2.5 rounded-full bg-white transition-transform duration-200"
                  :class="form.top_p_enabled ? 'translate-x-3.5' : 'translate-x-0.75'"
                />
              </button>
            </div>
            <input
              v-model.number="form.top_p"
              type="number"
              step="0.1"
              min="0"
              max="1"
              :disabled="!form.top_p_enabled"
              class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200 disabled:opacity-50"
            />
          </div>
        </div>

        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.reasoningEffort') }}</label>
          <select v-model="form.reasoning_effort" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none">
            <option value="none">none</option>
            <option value="minimal">minimal</option>
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
            <option value="xhigh">xhigh</option>
            <option value="max">max</option>
          </select>
        </div>

        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('search.prompt') }}</label>
          <textarea
            v-model="form.prompt_template"
            rows="3"
            class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring resize-none transition-shadow duration-200"
          />
          <p class="text-xs text-muted-foreground mt-1">{{ t('search.promptHint') }}</p>
        </div>

        <div class="flex items-center gap-2">
          <label class="text-xs text-muted-foreground">{{ t('search.enabled') }}</label>
          <button
            class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200"
            :class="form.enabled ? 'bg-primary' : 'bg-muted-foreground/30'"
            @click="form.enabled = !form.enabled"
          >
            <span
              class="inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform duration-200"
              :class="form.enabled ? 'translate-x-[18px]' : 'translate-x-[3px]'"
            />
          </button>
        </div>
      </div>

      <p v-if="saveError" class="text-xs text-destructive">{{ saveError }}</p>

      <div class="flex justify-end gap-2 pt-2">
        <button
          class="rounded-md border border-border bg-background text-foreground px-4 py-2 text-sm hover:bg-muted transition-all duration-200"
          @click="closeForm"
        >
          {{ t('model.cancel') }}
        </button>
        <button
          class="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 transition-all duration-200"
          @click="handleSave"
        >
          {{ t('model.save') }}
        </button>
      </div>
    </template>
  </div>
</template>
