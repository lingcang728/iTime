<script setup lang="ts">
import logoUrl from '../assets/logo.svg'

withDefaults(defineProps<{ size?: number }>(), { size: 34 })
</script>

<template>
  <span class="app-mark" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true">
    <img :src="logoUrl" alt="" draggable="false" />
  </span>
</template>

<style scoped>
.app-mark {
  display: block;
  flex: 0 0 auto;
}

.app-mark img {
  display: block;
  width: 100%;
  height: 100%;
  user-select: none;
}
</style>
