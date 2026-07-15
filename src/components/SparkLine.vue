<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{ values: number[]; color?: string }>()
const points = computed(() => {
  const max = Math.max(1, ...props.values)
  const min = Math.min(...props.values)
  const range = Math.max(1, max - min)
  return props.values.map((value, index) => `${index / Math.max(1, props.values.length - 1) * 100},${38 - ((value - min) / range) * 32}`).join(' ')
})
</script>

<template>
  <svg class="sparkline" viewBox="0 0 100 42" preserveAspectRatio="none" aria-hidden="true">
    <polyline :points="points" fill="none" :stroke="color ?? 'var(--accent-blue)'" stroke-width="2" vector-effect="non-scaling-stroke" />
  </svg>
</template>

