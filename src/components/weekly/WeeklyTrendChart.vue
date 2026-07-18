<script setup lang="ts">
import { computed, ref } from 'vue'

export interface TrendPoint {
  label: string
  note: string
  value: number | null
}

const props = defineProps<{ points: TrendPoint[] }>()
const activeIndex = ref<number | null>(null)
const values = computed(() => props.points.flatMap((point) => point.value === null ? [] : [point.value]))
const maximum = computed(() => values.value.reduce((current, value) => Math.max(current, value), 1))
const positioned = computed(() => props.points.map((point, index) => ({
  ...point,
  x: 7 + index / Math.max(1, props.points.length - 1) * 86,
  y: point.value === null ? null : 42 - point.value / maximum.value * 32,
})))
const path = computed(() => positioned.value.reduce((result, point, index) => {
  if (point.y === null) return result
  const previous = positioned.value[index - 1]
  const command = index === 0 || previous?.y === null ? 'M' : 'L'
  return `${result} ${command}${point.x.toFixed(2)},${point.y.toFixed(2)}`
}, '').trim())
const active = computed(() => activeIndex.value === null ? null : positioned.value[activeIndex.value])
const availableIndexes = computed(() => positioned.value.flatMap((point, index) => point.value === null ? [] : [index]))

function isEndpoint(index: number): boolean {
  return index === availableIndexes.value[0] || index === availableIndexes.value.at(-1)
}

function valueLabel(value: number | null): string {
  return value === null ? '暂无数据' : `${value.toFixed(1)} 小时`
}
</script>

<template>
  <div class="trend" @mouseleave="activeIndex = null">
    <div class="trend__legend"><i></i><span>主动注意力</span><small>悬停端点查看每日数值</small></div>
    <div v-if="values.length" class="trend__plot">
      <svg viewBox="0 0 100 50" preserveAspectRatio="none" role="img" aria-label="本周主动注意力折线图">
        <line v-for="y in [10, 26, 42]" :key="y" x1="6" :y1="y" x2="94" :y2="y" class="trend__grid" />
        <path :d="path" class="trend__line" />
        <g
          v-for="(point, index) in positioned"
          :key="`${point.label}-${point.note}`"
          :tabindex="point.value === null ? -1 : 0"
          role="img"
          :aria-label="`${point.label} ${point.note}，主动注意力 ${valueLabel(point.value)}`"
          @mouseenter="activeIndex = index"
          @focus="activeIndex = index"
          @blur="activeIndex = null"
        >
          <circle v-if="point.y !== null" :cx="point.x" :cy="point.y" :r="isEndpoint(index) ? 2.2 : 1.65" :class="{ endpoint: isEndpoint(index) }" />
        </g>
      </svg>
      <div
        v-if="active && active.y !== null"
        class="trend__tooltip"
        :style="{ left: `${active.x}%`, top: `${active.y / 50 * 100}%` }"
        role="tooltip"
      ><strong>{{ active.label }}</strong><span>{{ valueLabel(active.value) }}</span></div>
    </div>
    <div v-else class="trend__empty">本周尚无可验证的主动注意力记录</div>
    <div class="trend__labels" aria-hidden="true"><span v-for="point in points" :key="point.note">{{ point.label }}</span></div>
  </div>
</template>

<style scoped>
.trend { min-width: 0; margin-top: 14px; }
.trend__legend { display: flex; align-items: center; gap: 6px; color: var(--text-secondary); font-size: 10px; }
.trend__legend i { width: 16px; height: 4px; border-radius: 99px; background: var(--accent-green); }
.trend__legend small { margin-left: auto; color: var(--text-muted); font-size: 10px; }
.trend__plot { position: relative; height: 100px; margin-top: 4px; }
.trend__plot svg { width: 100%; height: 100%; overflow: visible; }
.trend__grid { stroke: color-mix(in srgb, var(--border-soft) 76%, transparent); stroke-width: .4; vector-effect: non-scaling-stroke; }
.trend__line { fill: none; stroke: var(--accent-green); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; vector-effect: non-scaling-stroke; }
.trend circle { fill: var(--bg-surface, var(--bg-card)); stroke: var(--accent-green); stroke-width: 1.7; vector-effect: non-scaling-stroke; cursor: pointer; transition: r 160ms ease, fill 160ms ease; }
.trend circle.endpoint { fill: var(--accent-green); }
.trend g:hover circle,
.trend g:focus-visible circle { r: 2.6; fill: var(--accent-green); }
.trend g:focus-visible circle { stroke: var(--border-focus); stroke-width: 3; }
.trend__tooltip { position: absolute; z-index: 3; display: grid; gap: 1px; transform: translate(-50%, -110%); padding: 6px 8px; border: 1px solid var(--border-soft); border-radius: 7px; color: var(--text-primary); background: color-mix(in srgb, var(--bg-card) 96%, transparent); box-shadow: var(--shadow-popover); font-size: 10px; pointer-events: none; white-space: nowrap; }
.trend__tooltip span { color: var(--text-secondary); }
.trend__labels { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); color: var(--text-muted); font-size: 10px; text-align: center; }
.trend__empty { height: 100px; display: grid; place-items: center; margin-top: 4px; border-radius: 9px; color: var(--text-muted); background: var(--bg-subtle); font-size: 10px; }
</style>
