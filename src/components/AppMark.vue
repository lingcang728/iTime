<script setup lang="ts">
import { PhHourglassHigh } from '@phosphor-icons/vue'

withDefaults(defineProps<{ size?: number }>(), { size: 34 })
</script>

<template>
  <span class="app-mark" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true">
    <PhHourglassHigh :size="Math.round(size * 0.64)" weight="regular" />
  </span>
</template>

<style scoped>
.app-mark {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid rgba(255, 255, 255, .18);
  border-radius: 8px;
  color: #f4f6f8;
  background: #131a22;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, .08), 0 3px 8px rgba(20, 28, 37, .16);
}

:global(html[data-theme="dark"]) .app-mark {
  color: #f2f5f7;
  background: #11161c;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, .1), 0 3px 10px rgba(0, 0, 0, .24);
}
</style>
