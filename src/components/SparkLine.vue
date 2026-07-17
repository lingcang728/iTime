<script setup lang="ts">
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{
  values: number[]
  color?: string
  labels?: string[]
  valueSuffix?: string
  ariaLabel?: string
}>(), {
  color: 'var(--accent-blue)',
  labels: () => [],
  valueSuffix: '',
  ariaLabel: '数据趋势',
})

const hoveredIndex = ref<number | null>(null)
const focusedIndex = ref<number | null>(null)
const activeIndex = computed(() => focusedIndex.value ?? hoveredIndex.value)
const numberFormatter = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 1 })

const chartPoints = computed(() => {
  if (!props.values.length) return []
  const maximum = Math.max(...props.values)
  const minimum = Math.min(...props.values)
  const range = Math.max(1, maximum - minimum)
  const divisor = Math.max(1, props.values.length - 1)
  return props.values.map((value, index) => ({
    index,
    value,
    x: index / divisor * 100,
    y: 37 - (value - minimum) / range * 30,
  }))
})

const linePoints = computed(() => chartPoints.value.map((point) => `${point.x},${point.y}`).join(' '))
const activePoint = computed(() => activeIndex.value === null ? null : chartPoints.value[activeIndex.value] ?? null)
const activeLabel = computed(() => {
  const point = activePoint.value
  if (!point) return ''
  const label = props.labels[point.index] ?? `第 ${point.index + 1} 项`
  return `${label} · ${numberFormatter.format(point.value)}${props.valueSuffix}`
})
const tooltipStyle = computed(() => {
  const point = activePoint.value
  if (!point) return {}
  return {
    left: `${Math.min(92, Math.max(8, point.x))}%`,
    top: `${point.y / 42 * 100}%`,
  }
})

function accessibleLabel(index: number): string {
  const label = props.labels[index] ?? `第 ${index + 1} 项`
  return `${label}，${numberFormatter.format(props.values[index] ?? 0)}${props.valueSuffix}`
}
</script>

<template>
  <div class="spark-chart" role="group" :aria-label="ariaLabel" @mouseleave="hoveredIndex = null">
    <svg viewBox="0 0 100 42" preserveAspectRatio="none" aria-hidden="true">
      <polyline
        v-if="linePoints"
        :points="linePoints"
        fill="none"
        :stroke="color"
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        vector-effect="non-scaling-stroke"
      />
    </svg>
    <button
      v-for="point in chartPoints"
      :key="point.index"
      class="spark-point"
      :class="{ endpoint: point.index === 0 || point.index === chartPoints.length - 1 }"
      type="button"
      :style="{ left: `${point.x}%`, top: `${point.y / 42 * 100}%`, '--point-color': color }"
      :aria-label="accessibleLabel(point.index)"
      @focus="focusedIndex = point.index"
      @blur="focusedIndex = null"
      @mouseenter="hoveredIndex = point.index"
    />
    <span v-if="activePoint" class="spark-tooltip" role="tooltip" :style="tooltipStyle">{{ activeLabel }}</span>
    <span v-if="!values.length" class="spark-empty">暂无趋势数据</span>
  </div>
</template>

<style scoped>
.spark-chart {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 46px;
}

svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: visible;
}

.spark-point {
  --point-color: var(--accent-blue);
  position: absolute;
  z-index: 1;
  width: 12px;
  height: 12px;
  padding: 0;
  transform: translate(-50%, -50%);
  border: 4px solid var(--bg-card);
  border-radius: 50%;
  background: var(--point-color);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--point-color) 52%, var(--border-strong));
  cursor: crosshair;
  opacity: .7;
  transition: transform 150ms var(--ease-out), opacity 150ms var(--ease-out);
}

.spark-point.endpoint { opacity: 1; }
.spark-point:hover,
.spark-point:focus-visible { z-index: 3; transform: translate(-50%, -50%) scale(1.28); opacity: 1; }

.spark-tooltip {
  position: absolute;
  z-index: 4;
  max-width: 180px;
  padding: 6px 8px;
  transform: translate(-50%, calc(-100% - 10px));
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  color: var(--text-primary);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-card);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  line-height: 1.3;
  white-space: nowrap;
  pointer-events: none;
}

.spark-empty {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  font-size: 11px;
}
</style>
