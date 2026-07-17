<script setup lang="ts">
defineProps<{
  src: string
  size?: number | string
  alt?: string
}>()
</script>

<template>
  <img
    class="ui-icon"
    :src="src"
    :alt="alt ?? ''"
    :width="size ?? 24"
    :height="size ?? 24"
    draggable="false"
    aria-hidden="true"
  />
</template>

<style scoped>
.ui-icon {
  display: block;
  flex: 0 0 auto;
  object-fit: contain;
  user-select: none;
  pointer-events: none;
}
</style>
