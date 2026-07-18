<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), { size: 34 })
</script>

<template>
  <svg class="app-mark" :width="size" :height="size" viewBox="0 0 36 36" aria-hidden="true">
    <circle cx="18" cy="19" r="13" fill="none" stroke="currentColor" stroke-width="2.25" />
    <path d="M18 11v8l5 3" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.25" />
    <path d="M11 4.8 8.6 7.2M25 4.8l2.4 2.4" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="2.25" />
    <circle class="app-mark__accent" cx="28.3" cy="8.2" r="2.35" />
  </svg>
</template>

<style scoped>
.app-mark {
  display: block;
  color: var(--text-secondary);
}

.app-mark__accent {
  fill: var(--accent);
}
</style>
