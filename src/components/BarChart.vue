<script setup lang="ts">
interface ChartPoint { label: string; value: number; secondary?: number; note?: string }
const props = defineProps<{ points: ChartPoint[]; unit?: string; compact?: boolean }>()
const maxValue = Math.max(1, ...props.points.flatMap((point) => [point.value, point.secondary ?? 0]))
</script>

<template>
  <div class="bar-chart" :class="{ 'bar-chart--compact': compact }">
    <div v-for="point in points" :key="point.label" class="bar-chart__item">
      <div class="bar-chart__value">{{ point.value.toFixed(1) }}<span v-if="unit">{{ unit }}</span></div>
      <div class="bar-chart__bars">
        <span class="bar primary" :style="{ height: `${Math.max(8, point.value / maxValue * 100)}%` }"></span>
        <span v-if="point.secondary !== undefined" class="bar secondary" :style="{ height: `${Math.max(8, point.secondary / maxValue * 100)}%` }"></span>
      </div>
      <span>{{ point.label }}</span>
      <small v-if="point.note">{{ point.note }}</small>
    </div>
  </div>
</template>

