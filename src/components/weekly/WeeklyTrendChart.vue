<script setup lang="ts">
import { computed, ref } from 'vue'

export interface TrendPoint {
  label: string
  note: string
  attention: number | null
  ai: number | null
}

const props = withDefaults(defineProps<{ points: TrendPoint[]; secondaryLabel?: string }>(), { secondaryLabel: 'AI 前台活跃' })
const activeIndex = ref<number | null>(null)
const values = computed(() => props.points.flatMap((point) => [point.attention, point.ai]
  .filter((value): value is number => value !== null)))
const maximum = computed(() => values.value.reduce((current, value) => Math.max(current, value), 1))
const positioned = computed(() => props.points.map((point, index) => ({
  ...point,
  x: 46 + index / Math.max(1, props.points.length - 1) * 608,
  attentionY: point.attention === null ? null : 184 - point.attention / maximum.value * 140,
  aiY: point.ai === null ? null : 184 - point.ai / maximum.value * 140,
})))
const attentionPath = computed(() => buildPath('attentionY'))
const aiPath = computed(() => buildPath('aiY'))
const active = computed(() => activeIndex.value === null ? null : positioned.value[activeIndex.value])

function buildPath(field: 'attentionY' | 'aiY'): string {
  return positioned.value.reduce((result, point, index) => {
    const y = point[field]
    if (y === null) return result
    const previous = positioned.value[index - 1]
    const command = index === 0 || previous?.[field] === null ? 'M' : 'L'
    return `${result} ${command}${point.x.toFixed(2)},${y.toFixed(2)}`
  }, '').trim()
}

function isEndpoint(index: number, field: 'attention' | 'ai'): boolean {
  const indexes = positioned.value.flatMap((point, pointIndex) => point[field] === null ? [] : [pointIndex])
  return index === indexes[0] || index === indexes.at(-1)
}

function valueLabel(value: number | null): string {
  return value === null ? '未记录' : `${value.toFixed(1)} 小时`
}

function tooltipTop(point: (typeof positioned.value)[number]): number {
  const candidates = [point.attentionY, point.aiY].filter((value): value is number => value !== null)
  return candidates.length ? Math.min(...candidates) : 110
}
</script>

<template>
  <div class="trend" @mouseleave="activeIndex = null">
    <div class="trend__legend">
      <span><i class="attention"></i>主动注意力</span>
      <span><i class="ai"></i>{{ secondaryLabel }}</span>
      <small>悬停或聚焦查看每日数值</small>
    </div>
    <div v-if="values.length" class="trend__plot">
      <svg viewBox="0 0 700 220" preserveAspectRatio="xMidYMid meet" role="img" aria-label="本周主动注意力与 AI 前台活跃趋势图">
        <line v-for="y in [44, 114, 184]" :key="y" x1="46" :y1="y" x2="654" :y2="y" class="trend__grid" />
        <path :d="attentionPath" class="trend__line trend__line--attention" />
        <path :d="aiPath" class="trend__line trend__line--ai" />
        <g
          v-for="(point, index) in positioned"
          :key="`${point.label}-${point.note}`"
          :tabindex="point.attention === null && point.ai === null ? -1 : 0"
          role="img"
          :aria-label="`${point.label} ${point.note}，主动注意力 ${valueLabel(point.attention)}，${secondaryLabel} ${valueLabel(point.ai)}`"
          @mouseenter="activeIndex = index"
          @focus="activeIndex = index"
          @blur="activeIndex = null"
        >
          <circle v-if="point.attentionY !== null" :cx="point.x" :cy="point.attentionY" :r="isEndpoint(index, 'attention') ? 7 : 5" class="attention" :class="{ endpoint: isEndpoint(index, 'attention') }" />
          <circle v-if="point.aiY !== null" :cx="point.x" :cy="point.aiY" :r="isEndpoint(index, 'ai') ? 7 : 5" class="ai" :class="{ endpoint: isEndpoint(index, 'ai') }" />
        </g>
      </svg>
      <div
        v-if="active"
        class="trend__tooltip"
        :style="{ left: `${active.x / 700 * 100}%`, top: `${tooltipTop(active) / 220 * 100}%` }"
        role="tooltip"
      >
        <strong>{{ active.label }} {{ active.note }}</strong>
        <span>主动注意力 {{ valueLabel(active.attention) }}</span>
        <span>{{ secondaryLabel }} {{ valueLabel(active.ai) }}</span>
      </div>
    </div>
    <div v-else class="trend__empty">本周尚无可验证的注意力或 {{ secondaryLabel }}记录</div>
    <div class="trend__labels" aria-hidden="true"><span v-for="point in points" :key="point.note">{{ point.label }}</span></div>
  </div>
</template>

<style scoped>
.trend { min-width: 0; margin-top: 12px; }
.trend__legend { display: flex; align-items: center; gap: 14px; color: var(--text-secondary); font-size: 10px; }
.trend__legend span { display: inline-flex; align-items: center; gap: 6px; }
.trend__legend i { width: 18px; height: 3px; border-radius: 99px; }
.trend__legend i.attention { background: var(--accent-green); }
.trend__legend i.ai { background: var(--accent-strong); }
.trend__legend small { margin-left: auto; color: var(--text-muted); font-size: 10px; }
.trend__plot { position: relative; width: 100%; min-height: 172px; aspect-ratio: 700 / 220; margin-top: 6px; }
.trend__plot svg { width: 100%; height: 100%; display: block; overflow: visible; }
.trend__grid { stroke: color-mix(in srgb, var(--border-soft) 76%, transparent); stroke-width: 1; vector-effect: non-scaling-stroke; }
.trend__line { fill: none; stroke-width: 2.2; stroke-linecap: round; stroke-linejoin: round; vector-effect: non-scaling-stroke; }
.trend__line--attention { stroke: var(--accent-green); }
.trend__line--ai { stroke: var(--accent-strong); stroke-dasharray: 6 5; }
.trend circle { fill: var(--bg-surface, var(--bg-card)); stroke-width: 2; vector-effect: non-scaling-stroke; cursor: pointer; transition: r 160ms ease, fill 160ms ease; }
.trend circle.attention { stroke: var(--accent-green); }
.trend circle.ai { stroke: var(--accent-strong); }
.trend circle.attention.endpoint { fill: var(--accent-green); }
.trend circle.ai.endpoint { fill: var(--accent-strong); }
.trend g:hover circle,
.trend g:focus-visible circle { r: 8; }
.trend g:focus-visible circle { stroke: var(--border-focus); stroke-width: 4; }
.trend__tooltip { position: absolute; z-index: 3; display: grid; gap: 2px; transform: translate(-50%, -112%); padding: 7px 9px; border: 1px solid var(--border-soft); border-radius: 7px; color: var(--text-primary); background: color-mix(in srgb, var(--bg-card) 96%, transparent); box-shadow: var(--shadow-popover); font-size: 10px; pointer-events: none; white-space: nowrap; }
.trend__tooltip span { color: var(--text-secondary); }
.trend__labels { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); margin-top: 2px; color: var(--text-muted); font-size: 10px; text-align: center; }
.trend__empty { min-height: 172px; display: grid; place-items: center; margin-top: 6px; border-radius: 9px; color: var(--text-muted); background: var(--bg-subtle); font-size: 10px; }

@media (max-width: 720px) {
  .trend__legend { flex-wrap: wrap; }
  .trend__legend small { width: 100%; margin-left: 0; }
  .trend__plot { min-height: 150px; }
}
</style>
