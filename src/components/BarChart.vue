<script setup lang="ts">
import { computed, ref } from 'vue'

interface ChartPoint {
  label: string
  value: number | null
  secondary?: number | null
  note?: string
}

const props = withDefaults(defineProps<{
  points: ChartPoint[]
  unit?: string
  compact?: boolean
  tone?: 'blue' | 'green'
  primaryLabel?: string
  secondaryLabel?: string
}>(), {
  unit: '',
  compact: false,
  tone: 'green',
  primaryLabel: '电脑活动',
  secondaryLabel: 'AI 前台活跃',
})

const activeIndex = ref<number | null>(null)
const maximum = computed(() => props.points.reduce(
  (current, point) => Math.max(current, point.value ?? 0, point.secondary ?? 0),
  1,
))
const active = computed(() => activeIndex.value === null ? null : props.points[activeIndex.value])

function height(value: number | null | undefined): string {
  if (!value) return '0%'
  return `${Math.max(4, value / maximum.value * 100)}%`
}

function valueLabel(value: number | null | undefined): string {
  return value === null || value === undefined ? '暂无数据' : `${value.toFixed(1)}${props.unit ? ` ${props.unit}` : ''}`
}
</script>

<template>
  <div class="week-bars" :class="[`week-bars--${tone}`, { 'week-bars--compact': compact }]">
    <div class="week-bars__plot" role="list" aria-label="按日统计柱状图">
      <button
        v-for="(point, index) in points"
        :key="`${point.label}-${point.note}`"
        type="button"
        class="week-bars__item"
        :aria-label="`${point.label} ${point.note ?? ''}，${primaryLabel} ${valueLabel(point.value)}${point.secondary !== undefined ? `，${secondaryLabel} ${valueLabel(point.secondary)}` : ''}`"
        @mouseenter="activeIndex = index"
        @mouseleave="activeIndex = null"
        @focus="activeIndex = index"
        @blur="activeIndex = null"
      >
        <span class="week-bars__value"><i>{{ point.value === null ? '—' : point.value.toFixed(1) }}</i><i v-if="point.secondary !== undefined" class="secondary">{{ point.secondary === null ? '—' : point.secondary.toFixed(1) }}</i></span>
        <span class="week-bars__columns">
          <i class="week-bars__bar week-bars__bar--primary" :style="{ height: height(point.value) }"></i>
          <i v-if="point.secondary !== undefined" class="week-bars__bar week-bars__bar--secondary" :style="{ height: height(point.secondary) }"></i>
        </span>
        <strong>{{ point.label }}</strong>
        <small>{{ point.note }}</small>
      </button>
    </div>
    <div class="week-bars__readout" aria-live="polite">
      <template v-if="active">
        <strong>{{ active.label }} · {{ active.note }}</strong>
        <span>{{ primaryLabel }} {{ valueLabel(active.value) }}</span>
        <span v-if="active.secondary !== undefined">{{ secondaryLabel }} {{ valueLabel(active.secondary) }}</span>
      </template>
      <span v-else>悬停或聚焦柱形查看当天数据</span>
    </div>
  </div>
</template>

<style scoped>
.week-bars { min-width: 0; }
.week-bars__plot {
  height: 184px;
  display: flex;
  align-items: stretch;
  gap: 14px;
  padding: 10px 12px 0;
  border-bottom: 1px solid var(--border-soft);
  background: repeating-linear-gradient(to top, transparent 0 44px, color-mix(in srgb, var(--border-soft) 70%, transparent) 45px);
}
.week-bars__item {
  min-width: 0;
  flex: 1;
  display: grid;
  grid-template-rows: 20px minmax(0, 1fr) 20px 16px;
  justify-items: center;
  padding: 0 2px;
  border: 0;
  border-radius: 8px 8px 0 0;
  color: var(--text-secondary);
  background: transparent;
  cursor: pointer;
}
.week-bars__item:hover,
.week-bars__item:focus-visible { background: color-mix(in srgb, var(--accent-green-soft) 42%, transparent); }
.week-bars__item:focus-visible { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.week-bars__value { width: 100%; display: flex; justify-content: center; gap: 5px; color: var(--text-primary); font-size: 10px; font-weight: 700; font-variant-numeric: tabular-nums; }
.week-bars__value i { width: min(28px, 38%); overflow: hidden; font-style: normal; text-align: center; }
.week-bars__value i.secondary { color: var(--text-muted); }
.week-bars__columns { width: 100%; display: flex; align-items: flex-end; justify-content: center; gap: 5px; }
.week-bars__bar { width: min(28px, 38%); border-radius: 6px 6px 2px 2px; transition: filter 160ms ease; }
.week-bars__bar--primary,
.week-bars--green .week-bars__bar--primary { background: var(--accent-green); }
.week-bars__bar--secondary { background: color-mix(in srgb, var(--text-muted) 62%, var(--bg-soft)); }
.week-bars__item:hover .week-bars__bar { filter: brightness(1.06); }
.week-bars__item strong { align-self: end; color: var(--text-primary); font-size: 10px; font-weight: 650; }
.week-bars__item small { color: var(--text-muted); font-size: 10px; }
.week-bars__readout { min-height: 26px; display: flex; align-items: center; gap: 14px; padding: 7px 12px 0; color: var(--text-muted); font-size: 10px; }
.week-bars__readout strong { color: var(--text-primary); }
.week-bars__readout span + span { padding-left: 14px; border-left: 1px solid var(--border-soft); }
.week-bars--compact .week-bars__plot { height: 168px; }
.week-bars--compact .week-bars__bar { width: min(22px, 35%); }
@media (max-width: 760px) {
  .week-bars__plot { gap: 5px; padding-inline: 4px; }
  .week-bars__readout { flex-wrap: wrap; gap: 5px 9px; }
}
</style>
