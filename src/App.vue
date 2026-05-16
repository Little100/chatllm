<script setup lang="ts">
import MainLayout from '@/components/layout/MainLayout.vue'
import { useTheme } from '@/composables/useTheme'
import { useI18n } from '@/composables/useI18n'

const { initTheme } = useTheme()
const { initLocale } = useI18n()
initTheme()
initLocale()
</script>

<template>
  <MainLayout />
</template>
