<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus, Pencil, Trash2 } from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

interface ModelConfig {
  id: string
  name: string
  provider: string
  model: string
  base_url: string | null
  max_tokens: number
  temperature: number
}

const emit = defineEmits<{
  add: []
  edit: [model: ModelConfig]
}>()

const models = ref<ModelConfig[]>([])

async function loadModels() {
  try {
    models.value = await invoke<ModelConfig[]>('list_model_configs')
  } catch (_) {}
}

async function deleteModel(id: string) {
  try {
    await invoke('delete_model_config', { id })
    models.value = models.value.filter((m) => m.id !== id)
  } catch (_) {}
}

onMounted(loadModels)
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-sm font-medium text-foreground">{{ t('settings.models') }}</h3>
      <button
        class="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:bg-primary/90 transition-all duration-200"
        @click="emit('add')"
      >
        <Plus class="h-3.5 w-3.5" />
        {{ t('model.add') }}
      </button>
    </div>

    <div class="space-y-2">
      <div
        v-for="model in models"
        :key="model.id"
        class="flex items-center justify-between rounded-lg border border-border p-3 hover:bg-muted/50 transition-all duration-150"
      >
        <div class="flex-1">
          <div class="text-sm font-medium text-foreground">{{ model.name }}</div>
          <div class="text-xs text-muted-foreground mt-0.5">
            {{ model.provider }} / {{ model.model }}
          </div>
        </div>
        <div class="flex items-center gap-1">
          <button
            class="rounded p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors duration-150"
            @click="emit('edit', model)"
          >
            <Pencil class="h-3.5 w-3.5" />
          </button>
          <button
            class="rounded p-1.5 text-muted-foreground hover:bg-destructive hover:text-destructive-foreground transition-colors duration-150"
            @click="deleteModel(model.id)"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <div v-if="models.length === 0" class="py-8 text-center text-sm text-muted-foreground">
        {{ t('model.noConfigs') }}
      </div>
    </div>
  </div>
</template>
