import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ModelConfig } from '@/types/model'

const modelConfigs = ref<ModelConfig[]>([])
const modelConfigId = ref('')
const localReasoning = ref('none')
const localTemperature = ref(0.7)
const localTemperatureEnabled = ref(true)
const localTopP = ref(1.0)
const localTopPEnabled = ref(true)
const localMaxTokens = ref(4096)
const loaded = ref(false)

const selectedConfig = computed(() =>
  modelConfigs.value.find(c => c.id === modelConfigId.value) || null
)

watch(selectedConfig, (cfg) => {
  if (!cfg) return
  localReasoning.value = cfg.reasoning_effort || 'none'
  localTemperature.value = cfg.temperature
  localTemperatureEnabled.value = cfg.temperature_enabled
  localTopP.value = cfg.top_p
  localTopPEnabled.value = cfg.top_p_enabled
  localMaxTokens.value = cfg.max_tokens
})

let saveTimer: ReturnType<typeof setTimeout> | null = null
function debouncedSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(persistParams, 600)
}

async function persistParams() {
  const cfg = selectedConfig.value
  if (!cfg) return
  try {
    await invoke('update_model_config', {
      id: cfg.id,
      params: {
        name: cfg.name,
        provider: cfg.provider,
        model: cfg.model,
        baseUrl: cfg.base_url || null,
        apiKeyName: cfg.api_key_name || null,
        apiKey: cfg.api_key || null,
        maxTokens: localMaxTokens.value,
        contextWindow: cfg.context_window,
        temperature: localTemperature.value,
        topP: localTopP.value,
        temperatureEnabled: localTemperatureEnabled.value,
        topPEnabled: localTopPEnabled.value,
        systemPrompt: cfg.system_prompt || null,
        reasoningEffort: localReasoning.value === 'none' ? null : localReasoning.value,
        passbackReasoning: cfg.passback_reasoning,
        retryCount: cfg.retry_count,
      },
    })
  } catch (_) {}
}

async function loadConfigs() {
  if (loaded.value) return
  try {
    modelConfigs.value = await invoke<ModelConfig[]>('list_model_configs')
    if (modelConfigs.value.length > 0 && !modelConfigId.value) {
      modelConfigId.value = modelConfigs.value[0].id
    }
    loaded.value = true
  } catch (_) {}
}

/// 强制重新加载配置列表, 修正被删除或新增后的 ID 失效问题
async function reloadConfigs() {
  try {
    modelConfigs.value = await invoke<ModelConfig[]>('list_model_configs')
    // 当前选中 ID 已不存在时回退到第一个
    const ids = modelConfigs.value.map(c => c.id)
    if (!ids.includes(modelConfigId.value) && modelConfigs.value.length > 0) {
      modelConfigId.value = modelConfigs.value[0].id
    }
    loaded.value = true
  } catch (_) {}
}

export function useModelSelection() {
  return {
    modelConfigs,
    modelConfigId,
    selectedConfig,
    localReasoning,
    localTemperature,
    localTemperatureEnabled,
    localTopP,
    localTopPEnabled,
    localMaxTokens,
    debouncedSave,
    loadConfigs,
    reloadConfigs,
  }
}
