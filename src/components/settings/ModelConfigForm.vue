<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { RefreshCw } from 'lucide-vue-next'
import ApiKeyField from './ApiKeyField.vue'
import { useI18n } from '@/composables/useI18n'
import type { ModelConfig } from '@/types/model'

const { t } = useI18n()

type ModelConfigForm = Omit<ModelConfig, 'id' | 'created_at' | 'updated_at'>

const props = defineProps<{ model?: ModelConfig | null }>()
const emit = defineEmits<{ save: []; cancel: [] }>()

const form = ref<ModelConfigForm>({
  name: '',
  provider: 'openai_compatible',
  model: '',
  base_url: '',
  api_key_name: '',
  api_key: '',
  max_tokens: 4096,
  context_window: 128000,
  temperature: 0.7,
  top_p: 1.0,
  temperature_enabled: true,
  top_p_enabled: true,
  system_prompt: '',
  reasoning_effort: 'none',
  passback_reasoning: false,
  retry_count: 3,
})

const apiKey = ref('')
const modelList = ref<string[]>([])
const fetching = ref(false)
const fetchError = ref('')
const saveError = ref('')
const apiKeyMissing = ref(false)

onMounted(() => {
  if (props.model) {
    form.value = {
      name: props.model.name,
      provider: props.model.provider,
      model: props.model.model,
      base_url: props.model.base_url,
      api_key_name: props.model.api_key_name,
      api_key: props.model.api_key,
      max_tokens: props.model.max_tokens,
      context_window: props.model.context_window ?? 128000,
      temperature: props.model.temperature,
      top_p: props.model.top_p,
      temperature_enabled: props.model.temperature_enabled ?? true,
      top_p_enabled: props.model.top_p_enabled ?? true,
      system_prompt: props.model.system_prompt,
      reasoning_effort: props.model.reasoning_effort || 'none',
      passback_reasoning: props.model.passback_reasoning ?? false,
      retry_count: props.model.retry_count ?? 3,
    }
    if (props.model.api_key) {
      apiKey.value = props.model.api_key
    } else if (props.model.api_key_name) {
      invoke<string>('get_api_key', { service: props.model.api_key_name })
        .then((key) => { apiKey.value = key })
        .catch(() => { apiKeyMissing.value = true })
    }
  }
})

async function fetchModels() {
  if (!form.value.base_url || !apiKey.value) {
    fetchError.value = t.value('settings.fillBaseUrlAndKey')
    return
  }
  fetching.value = true
  fetchError.value = ''
  try {
    const list = await invoke<string[]>('fetch_model_list', {
      baseUrl: form.value.base_url,
      apiKey: apiKey.value,
    })
    modelList.value = list
  } catch (e: unknown) {
    fetchError.value = e instanceof Error ? e.message : String(e)
  } finally {
    fetching.value = false
  }
}

async function handleSave() {
  saveError.value = ''
  const isEdit = !!props.model?.id
  if (!isEdit && !apiKey.value.trim()) {
    saveError.value = t.value('settings.fillApiKey')
    return
  }
  const keyName = form.value.api_key_name || `model_${Date.now()}`

  const params = {
    name: form.value.name,
    provider: form.value.provider,
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
    systemPrompt: form.value.system_prompt || null,
    reasoningEffort: form.value.reasoning_effort === 'none' ? null : form.value.reasoning_effort,
    passbackReasoning: form.value.passback_reasoning,
    retryCount: form.value.retry_count,
  }

  try {
    if (props.model?.id) {
      await invoke('update_model_config', { id: props.model.id, params })
    } else {
      await invoke('create_model_config', { params })
    }
    apiKeyMissing.value = false
    emit('save')
  } catch (e: unknown) {
    console.error(e)
    saveError.value = e instanceof Error ? e.message : String(e)
  }
}
</script>

<template>
  <div class="space-y-4">
    <h3 class="text-sm font-medium text-foreground">
      {{ model?.id ? t('model.edit') : t('model.add') }}
    </h3>

    <div class="grid gap-3">
      <div>
        <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.name') }}</label>
        <input v-model="form.name" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200" />
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.provider') }}</label>
          <select v-model="form.provider" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none">
            <option value="openai_compatible">OpenAI Compatible</option>
            <option value="anthropic">Anthropic</option>
            <option value="google">Google</option>
          </select>
        </div>
        <div>
          <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.modelId') }}</label>
          <div class="flex gap-1">
            <input
              v-model="form.model"
              list="model-list-options"
              class="flex-1 rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200"
              placeholder="gpt-4o"
            />
            <datalist id="model-list-options">
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
      </div>

      <p v-if="fetchError" class="text-xs text-destructive">{{ fetchError }}</p>

      <div>
        <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.baseUrl') }}</label>
        <input v-model="form.base_url" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200" placeholder="https://api.openai.com" />
      </div>

      <div>
        <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.apiKey') }}</label>
        <ApiKeyField v-model="apiKey" placeholder="sk-..." />
        <p v-if="apiKeyMissing" class="text-xs text-destructive mt-1">
          系统密钥库中未找到 API Key, 请重新填写并保存
        </p>
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
        <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.retryCount') }}</label>
        <input v-model.number="form.retry_count" type="number" min="1" max="10" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-shadow duration-200" />
      </div>

      <label class="flex items-start gap-2">
        <input type="checkbox" v-model="form.passback_reasoning" class="mt-1" />
        <div class="flex flex-col">
          <span class="text-sm text-foreground">{{ t('model.passbackReasoning') }}</span>
          <span class="text-xs text-muted-foreground">{{ t('model.passbackReasoningHelp') }}</span>
        </div>
      </label>

      <div>
        <label class="text-xs text-muted-foreground mb-1 block">{{ t('model.systemPrompt') }}</label>
        <textarea v-model="form.system_prompt" rows="3" class="w-full rounded-md border border-input bg-background text-foreground px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring resize-none transition-shadow duration-200" />
      </div>
    </div>

    <p v-if="saveError" class="text-xs text-destructive">{{ saveError }}</p>

    <div class="flex justify-end gap-2 pt-2">
      <button
        class="rounded-md border border-border bg-background text-foreground px-4 py-2 text-sm hover:bg-muted transition-all duration-200"
        @click="emit('cancel')"
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
  </div>
</template>
