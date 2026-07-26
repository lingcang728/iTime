<script setup lang="ts">
import { computed, ref } from 'vue'

export interface InputTrendPoint {
  label: string
  accessibleLabel: string
  value: number
}

const props = defineProps<{
  points: InputTrendPoint[]
  mode: 'line' | 'bar'
  ariaLabel: string
}>()

const hoveredIndex = ref<number | null>(null)
const focusedIndex = ref<number | null>(null)
const activeIndex = computed(() => focusedIndex.value ?? hoveredIndex.value)
const activePoint = computed(() => activeIndex.value === null ? null : chartPoints.value[activeIndex.value] ?? null)
const numberFormatter = new Intl.NumberFormat('zh-CN')

function niceCeiling(value: number): number {
  if (value <= 0) return 4
  const magnitude = 10 ** Math.floor(Math.log10(value))
  const normalized = value / magnitude
  const factor = [1, 1.5, 2, 3, 4, 5, 6, 8, 10].find((candidate) => normalized <= candidate) ?? 10
  return factor * magnitude
}

const maximum = computed(() => niceCeiling(Math.max(0, ...props.points.map((point) => point.value))))
const yTicks = computed(() => [maximum.value, maximum.value * .75, maximum.value * .5, maximum.value * .25, 0])
const barWidth = computed(() => Math.min(6.6, Math.max(1.5, 64 / Math.max(1, props.points.length))))
const horizontalInset = computed(() => Math.max(1.6, barWidth.value / 2 + .5))
const chartPoints = computed(() => {
  const divisor = Math.max(1, props.points.length - 1)
  const availableWidth = 100 - horizontalInset.value * 2
  return props.points.map((point, index) => ({
    ...point,
    index,
    x: props.points.length === 1 ? 50 : horizontalInset.value + index / divisor * availableWidth,
    y: 46 - point.value / maximum.value * 40,
  }))
})
const linePath = computed(() => {
  const points = chartPoints.value
  if (!points.length) return ''
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`
  let path = `M ${points[0].x} ${points[0].y}`
  for (let index = 0; index < points.length - 1; index += 1) {
    const previous = points[Math.max(0, index - 1)]
    const current = points[index]
    const next = points[index + 1]
    const following = points[Math.min(points.length - 1, index + 2)]
    const firstControlX = current.x + (next.x - previous.x) / 6
    const firstControlY = Math.min(46, Math.max(6, current.y + (next.y - previous.y) / 6))
    const secondControlX = next.x - (following.x - current.x) / 6
    const secondControlY = Math.min(46, Math.max(6, next.y - (following.y - current.y) / 6))
    path += ` C ${firstControlX} ${firstControlY}, ${secondControlX} ${secondControlY}, ${next.x} ${next.y}`
  }
  return path
})
const areaPath = computed(() => {
  const points = chartPoints.value
  if (!points.length) return ''
  return `${linePath.value} L ${points.at(-1)?.x ?? 100} 46 L ${points[0].x} 46 Z`
})
const renderKey = computed(() => {
  const first = props.points[0]?.label ?? 'empty'
  const last = props.points.at(-1)?.label ?? 'empty'
  return `${props.mode}-${props.points.length}-${first}-${last}`
})
const activeTooltip = computed(() => {
  const point = activePoint.value
  return point ? `${point.accessibleLabel} · ${numberFormatter.format(point.value)} 次` : ''
})
const tooltipStyle = computed(() => {
  const point = activePoint.value
  if (!point) return {}
  return {
    left: `${Math.min(92, Math.max(8, point.x))}%`,
    top: `${point.y / 52 * 100}%`,
  }
})

function formatAxisValue(value: number): string {
  if (value >= 10_000) return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}k`
  if (value >= 1000) return `${(value / 1000).toFixed(1)}k`
  return numberFormatter.format(Math.round(value))
}

function pointAriaLabel(index: number): string {
  const point = props.points[index]
  return `${point?.accessibleLabel ?? `第 ${index + 1} 天`}，${numberFormatter.format(point?.value ?? 0)} 次字符键按下`
}
</script>

<template>
  <div class="input-trend-chart" :class="{ 'is-compact': points.length > 10 }" role="group" :aria-label="ariaLabel" @mouseleave="hoveredIndex = null">
    <div class="trend-y-axis" aria-hidden="true">
      <span v-for="tick in yTicks" :key="tick">{{ formatAxisValue(tick) }}</span>
    </div>
    <div class="trend-plot">
      <svg viewBox="0 0 100 52" preserveAspectRatio="none" aria-hidden="true">
        <line v-for="y in [6, 16, 26, 36, 46]" :key="y" x1="0" :y1="y" x2="100" :y2="y" class="trend-grid-line" />
        <Transition name="chart-swap">
          <g :key="renderKey" class="trend-series" :class="`is-${mode}`">
            <template v-if="mode === 'line'">
              <defs>
                <linearGradient id="input-trend-area" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="var(--input-chart-accent)" stop-opacity=".34" />
                  <stop offset="100%" stop-color="var(--input-chart-accent)" stop-opacity=".02" />
                </linearGradient>
              </defs>
              <path :d="areaPath" class="trend-area" />
              <path :d="linePath" class="trend-line" pathLength="1" />
            </template>
            <template v-else>
              <rect
                v-for="point in chartPoints"
                :key="point.index"
                class="trend-bar"
                :class="{ 'is-zero': point.value === 0 }"
                :x="point.x - barWidth / 2"
                :y="point.value === 0 ? 45.2 : point.y"
                :width="barWidth"
                :height="point.value === 0 ? .8 : 46 - point.y"
                rx="1.2"
                :style="{ '--bar-index': point.index }"
              />
            </template>
          </g>
        </Transition>
      </svg>

      <button
        v-for="point in chartPoints"
        :key="point.index"
        type="button"
        class="trend-point"
        :class="{ 'is-visible': mode === 'line', 'is-active': activeIndex === point.index }"
        :style="{ left: `${point.x}%`, top: `${point.y / 52 * 100}%` }"
        :aria-label="pointAriaLabel(point.index)"
        @focus="focusedIndex = point.index"
        @blur="focusedIndex = null"
        @mouseenter="hoveredIndex = point.index"
      />
      <Transition name="tooltip">
        <span v-if="activePoint" class="trend-tooltip" role="tooltip" :style="tooltipStyle">{{ activeTooltip }}</span>
      </Transition>
    </div>
    <div class="trend-x-axis" aria-hidden="true">
      <span
        v-for="point in chartPoints"
        :key="point.index"
        :class="{ 'is-hidden': points.length > 10 && point.index % 5 !== 0 && point.index !== points.length - 1 }"
        :style="{ left: `${point.x}%` }"
      >{{ point.label }}</span>
    </div>
  </div>
</template>

<style scoped>
.input-trend-chart {
  --input-chart-accent: #2f86df;
  --input-chart-point-fill: #f7f9fb;
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr);
  grid-template-rows: minmax(210px, 1fr) 26px;
  min-height: 258px;
}

.trend-y-axis {
  grid-row: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 8px 10px 18px 0;
  color: var(--text-muted);
  font: 500 11px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.trend-plot {
  position: relative;
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
}

.trend-plot svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.trend-grid-line {
  stroke: color-mix(in srgb, var(--border-strong) 54%, transparent);
  stroke-width: .22;
  stroke-dasharray: 1.4 1.4;
  vector-effect: non-scaling-stroke;
}

.trend-series {
  transform-box: fill-box;
  transform-origin: center bottom;
}

.trend-area { fill: url(#input-trend-area); }

.trend-line {
  fill: none;
  stroke: var(--input-chart-accent);
  stroke-width: 2.7;
  stroke-linecap: round;
  stroke-linejoin: round;
  vector-effect: non-scaling-stroke;
  animation: draw-line 520ms var(--ease-out) both;
}

.trend-bar {
  fill: color-mix(in srgb, var(--input-chart-accent) 84%, var(--bg-card));
  transform-box: fill-box;
  transform-origin: center bottom;
  animation: rise-bar 380ms var(--ease-out) both;
  animation-delay: min(calc(var(--bar-index) * 14ms), 180ms);
}

.trend-bar.is-zero { fill: color-mix(in srgb, var(--input-chart-accent) 30%, var(--bg-soft)); }

.trend-point {
  position: absolute;
  z-index: 2;
  width: 20px;
  height: 20px;
  padding: 0;
  transform: translate(-50%, -50%);
  border: 0;
  border-radius: 50%;
  background: transparent;
  cursor: crosshair;
}

.trend-point::after {
  content: '';
  position: absolute;
  inset: 3px;
  border: 2px solid var(--input-chart-accent);
  border-radius: inherit;
  background: var(--input-chart-point-fill);
  box-shadow: 0 0 0 .5px color-mix(in srgb, var(--input-chart-accent) 70%, transparent);
  opacity: 0;
  transform: scale(.76);
  transition: opacity 160ms ease, transform 180ms var(--ease-out);
}

.input-trend-chart.is-compact .trend-point::after {
  inset: 5px;
  border-width: 1.5px;
}

.trend-point.is-visible::after,
.trend-point:hover::after,
.trend-point:focus-visible::after,
.trend-point.is-active::after {
  opacity: 1;
  transform: scale(1);
}

.trend-point:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

.trend-tooltip {
  position: absolute;
  z-index: 4;
  max-width: 200px;
  padding: 7px 9px;
  transform: translate(-50%, calc(-100% - 10px));
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-elevated) 96%, transparent);
  box-shadow: var(--shadow-popover);
  font: 600 11px/1.3 var(--font-data);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  pointer-events: none;
}

.trend-x-axis {
  position: relative;
  grid-column: 2;
  grid-row: 2;
  color: var(--text-muted);
  font: 500 11px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.trend-x-axis span {
  position: absolute;
  top: 8px;
  transform: translateX(-50%);
  white-space: nowrap;
}

.trend-x-axis span:first-child { transform: translateX(-28%); }
.trend-x-axis span:last-child { transform: translateX(-72%); }
.trend-x-axis span.is-hidden { display: none; }

.chart-swap-enter-active,
.chart-swap-leave-active { transition: opacity 180ms ease, transform 260ms var(--ease-out); }
.chart-swap-enter-from { opacity: 0; transform: scaleY(.9); }
.chart-swap-leave-to { opacity: 0; transform: scaleY(1.02); }
.tooltip-enter-active,
.tooltip-leave-active { transition: opacity 120ms ease, transform 160ms var(--ease-out); }
.tooltip-enter-from,
.tooltip-leave-to { opacity: 0; transform: translate(-50%, calc(-100% - 4px)); }

@keyframes draw-line {
  from { stroke-dasharray: 1; stroke-dashoffset: 1; opacity: .3; }
  to { stroke-dasharray: 1; stroke-dashoffset: 0; opacity: 1; }
}

@keyframes rise-bar {
  from { opacity: .25; transform: scaleY(.08); }
  to { opacity: 1; transform: scaleY(1); }
}

@media (max-width: 760px) {
  .input-trend-chart {
    grid-template-columns: 40px minmax(0, 1fr);
    min-height: 224px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .trend-line,
  .trend-bar { animation: none; }

  .chart-swap-enter-active,
  .chart-swap-leave-active,
  .tooltip-enter-active,
  .tooltip-leave-active { transition-duration: 1ms; }
}
</style>
