<script setup lang="ts">
</script>

<template>
  <span class="streaming-cursor inline-block w-1.5 h-4 bg-foreground/70 ml-0.5 align-middle" />
</template>

<style>
.streaming-cursor {
  animation: blink 1s step-end infinite;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
</style>
