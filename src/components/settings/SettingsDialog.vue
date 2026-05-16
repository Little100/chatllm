<script setup lang="ts">
import { ref } from 'vue'
import { X } from 'lucide-vue-next'
import ModelConfigList from './ModelConfigList.vue'
import ModelConfigForm from './ModelConfigForm.vue'
import GeneralSettings from './GeneralSettings.vue'
import SearchConfigForm from './SearchConfigForm.vue'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()
const emit = defineEmits<{ close: [] }>()

type Tab = 'models' | 'search' | 'general'
const activeTab = ref<Tab>('models')
const showModelForm = ref(false)
const editingModel = ref<any>(null)

function openModelForm(model?: any) {
  editingModel.value = model || null
  showModelForm.value = true
}

function closeModelForm() {
  showModelForm.value = false
  editingModel.value = null
}

const tabLabels: Record<Tab, string> = {
  models: 'settings.models',
  search: 'search.title',
  general: 'settings.general',
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-in fade-in duration-200">
    <div class="relative w-full max-w-2xl max-h-[80vh] rounded-xl border border-border bg-background shadow-xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
      <!-- 头部 -->
      <div class="flex items-center justify-between border-b border-border px-6 py-4">
        <h2 class="text-lg font-semibold text-foreground">{{ t('settings.title') }}</h2>
        <button
          class="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors duration-150"
          @click="emit('close')"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <!-- Tab 导航 -->
      <div class="flex border-b border-border px-6">
        <button
          v-for="tab in (['models', 'search', 'general'] as const)"
          :key="tab"
          class="px-4 py-2.5 text-sm font-medium border-b-2 transition-all duration-200"
          :class="activeTab === tab ? 'border-primary text-foreground' : 'border-transparent text-muted-foreground hover:text-foreground'"
          @click="activeTab = tab"
        >
          {{ t(tabLabels[tab]) }}
        </button>
      </div>

      <!-- 内容区 -->
      <div class="flex-1 overflow-y-auto p-6">
        <Transition name="scale-fade" mode="out-in">
          <ModelConfigForm
            v-if="activeTab === 'models' && showModelForm"
            key="model-form"
            :model="editingModel"
            @save="closeModelForm"
            @cancel="closeModelForm"
          />
          <ModelConfigList
            v-else-if="activeTab === 'models'"
            key="model-list"
            @add="openModelForm()"
            @edit="openModelForm"
          />
          <SearchConfigForm v-else-if="activeTab === 'search'" key="search" />
          <GeneralSettings v-else-if="activeTab === 'general'" key="general" />
        </Transition>
      </div>
    </div>
  </div>
</template>
