<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

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
const lineAnimationEpoch = ref(0)
const rangeMotion = ref<'idle' | 'expanding' | 'contracting'>('idle')
const numberFormatter = new Intl.NumberFormat('zh-CN')
const maximumBarWidth = 4.8
let previousPointCount = props.points.length
let rangeMotionTimer: number | null = null

const activeIndex = computed(() => focusedIndex.value ?? hoveredIndex.value)
const activePoint = computed(() => activeIndex.value === null ? null : chartPoints.value[activeIndex.value] ?? null)

function niceCeiling(value: number): number {
  if (value <= 0) return 4
  const magnitude = 10 ** Math.floor(Math.log10(value))
  const normalized = value / magnitude
  const factor = [1, 1.5, 2, 3, 4, 5, 6, 8, 10].find((candidate) => normalized <= candidate) ?? 10
  return factor * magnitude
}

const isDense = computed(() => props.points.length > 10)
const maximum = computed(() => niceCeiling(Math.max(0, ...props.points.map((point) => point.value))))
const yTicks = computed(() => [maximum.value, maximum.value * .75, maximum.value * .5, maximum.value * .25, 0])
const barWidth = computed(() => isDense.value ? 1.45 : 4.2)
const horizontalInset = computed(() => Math.max(1.8, barWidth.value / 2 + .8))
const chartPoints = computed(() => {
  const divisor = Math.max(1, props.points.length - 1)
  const availableWidth = 100 - horizontalInset.value * 2
  return props.points.map((point, index) => ({
    ...point,
    index,
    x: props.points.length === 1 ? 50 : horizontalInset.value + index / divisor * availableWidth,
    y: 46 - point.value / maximum.value * 40,
    barScale: Math.max(point.value === 0 ? .012 : .02, point.value / maximum.value),
  }))
})
const linePath = computed(() => {
  const points = chartPoints.value
  if (!points.length) return ''
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`
  let path = `M ${points[0].x} ${points[0].y}`
  for (let index = 0; index < points.length - 1; index += 1) {
    const current = points[index]
    const next = points[index + 1]
    const midpoint = (current.x + next.x) / 2
    path += ` C ${midpoint} ${current.y}, ${midpoint} ${next.y}, ${next.x} ${next.y}`
  }
  return path
})
const areaPath = computed(() => {
  const points = chartPoints.value
  if (!points.length) return ''
  return `${linePath.value} L ${points.at(-1)?.x ?? 100} 46 L ${points[0].x} 46 Z`
})
const pointSignature = computed(() => {
  const first = props.points[0]?.accessibleLabel ?? 'empty'
  const last = props.points.at(-1)?.accessibleLabel ?? 'empty'
  return `${props.points.length}-${first}-${last}`
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

watch(pointSignature, () => {
  const nextCount = props.points.length
  rangeMotion.value = nextCount > previousPointCount ? 'expanding' : 'contracting'
  previousPointCount = nextCount
  hoveredIndex.value = null
  focusedIndex.value = null
  if (props.mode === 'line') lineAnimationEpoch.value += 1
  if (rangeMotionTimer !== null) window.clearTimeout(rangeMotionTimer)
  rangeMotionTimer = window.setTimeout(() => {
    rangeMotion.value = 'idle'
    rangeMotionTimer = null
  }, 720)
})

watch(() => props.mode, (mode, previousMode) => {
  hoveredIndex.value = null
  focusedIndex.value = null
  if (mode === 'line' && previousMode !== 'line') lineAnimationEpoch.value += 1
})

onBeforeUnmount(() => {
  if (rangeMotionTimer !== null) window.clearTimeout(rangeMotionTimer)
})

function formatAxisValue(value: number): string {
  if (value >= 10_000) return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}k`
  if (value >= 1000) return `${(value / 1000).toFixed(1)}k`
  return numberFormatter.format(Math.round(value))
}

function formatBarValue(value: number): string {
  if (!isDense.value) return numberFormatter.format(value)
  if (value >= 10_000) return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}k`
  if (value >= 1000) return `${(value / 1000).toFixed(1)}k`
  return numberFormatter.format(value)
}

function shouldShowAxisLabel(index: number): boolean {
  return !isDense.value || index === 0 || index === props.points.length - 1 || index % 5 === 0
}

function shouldShowMarker(value: number): boolean {
  return !isDense.value || value > 0
}

function datumXStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    transform: `translate3d(${point.x}%, 0, 0)`,
    '--datum-delay': `${Math.min(point.index * (isDense.value ? 8 : 18), 180)}ms`,
  }
}

function datumPointStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    transform: `translate3d(${point.x}%, ${point.y / 52 * 100}%, 0)`,
    '--bar-width': String(barWidth.value),
    '--bar-hit-height': String(Math.max(3, (46 - point.y) / 52 * 100)),
  }
}

function datumBarStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    ...datumXStyle(point),
    '--bar-scale-x': String(barWidth.value / maximumBarWidth),
    '--bar-scale-y': String(point.barScale),
  }
}

function pointAriaLabel(index: number): string {
  const point = props.points[index]
  return `${point?.accessibleLabel ?? `第 ${index + 1} 天`}，${numberFormatter.format(point?.value ?? 0)} 次字符键按下`
}
</script>

<template>
  <div
    class="input-trend-chart"
    :class="[
      `is-${mode}`,
      {
        'is-dense': isDense,
        'is-range-expanding': rangeMotion === 'expanding',
        'is-range-contracting': rangeMotion === 'contracting',
      },
    ]"
    :data-range-motion="rangeMotion"
    role="group"
    :aria-label="ariaLabel"
    @mouseleave="hoveredIndex = null"
  >
    <div class="trend-y-axis" aria-hidden="true">
      <span v-for="tick in yTicks" :key="tick">{{ formatAxisValue(tick) }}</span>
    </div>

    <div class="trend-plot">
      <svg viewBox="0 0 100 52" preserveAspectRatio="none" aria-hidden="true">
        <defs>
          <linearGradient id="input-trend-area" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="var(--input-chart-accent)" stop-opacity=".24" />
            <stop offset="68%" stop-color="var(--input-chart-accent)" stop-opacity=".08" />
            <stop offset="100%" stop-color="var(--input-chart-accent)" stop-opacity=".015" />
          </linearGradient>
        </defs>
        <line v-for="y in [6, 16, 26, 36, 46]" :key="y" x1="0" :y1="y" x2="100" :y2="y" class="trend-grid-line" />
        <Transition name="line-layer" :duration="{ enter: 720, leave: 240 }" appear>
          <g v-if="mode === 'line'" :key="lineAnimationEpoch" class="trend-line-layer">
            <path :d="areaPath" class="trend-area" />
            <path :d="linePath" class="trend-line" />
          </g>
        </Transition>
      </svg>

      <TransitionGroup tag="div" name="datum" class="trend-bar-layer" aria-hidden="true">
        <div
          v-for="point in chartPoints"
          :key="point.accessibleLabel"
          class="trend-bar-node"
          :class="{ 'is-zero': point.value === 0, 'is-active': activeIndex === point.index }"
          :style="datumBarStyle(point)"
        >
          <span class="trend-bar" />
        </div>
      </TransitionGroup>

      <TransitionGroup tag="div" name="datum" class="trend-value-layer" aria-hidden="true">
        <div
          v-for="point in chartPoints.filter((item) => item.value > 0)"
          :key="point.accessibleLabel"
          class="trend-value-node"
          :style="{ ...datumPointStyle(point), '--datum-delay': `${Math.min(point.index * (isDense ? 8 : 18), 180)}ms` }"
        >
          <span class="trend-bar-value">{{ formatBarValue(point.value) }}</span>
        </div>
      </TransitionGroup>

      <TransitionGroup tag="div" name="datum" class="trend-hit-layer">
        <div
          v-for="point in chartPoints"
          :key="point.accessibleLabel"
          class="trend-hit-node"
          :data-datum-key="point.accessibleLabel"
          :style="datumPointStyle(point)"
        >
          <button
            type="button"
            class="trend-point"
            :class="{
              'is-marker-visible': mode === 'line' && shouldShowMarker(point.value),
              'is-active': activeIndex === point.index,
            }"
            :aria-label="pointAriaLabel(point.index)"
            @focus="focusedIndex = point.index"
            @blur="focusedIndex = null"
            @mouseenter="hoveredIndex = point.index"
          />
        </div>
      </TransitionGroup>

      <Transition name="tooltip">
        <span v-if="activePoint" class="trend-tooltip" role="tooltip" :style="tooltipStyle">{{ activeTooltip }}</span>
      </Transition>
    </div>

    <TransitionGroup tag="div" name="datum" class="trend-x-axis" aria-hidden="true">
      <span
        v-for="point in chartPoints"
        :key="point.accessibleLabel"
        class="trend-x-node"
        :style="datumXStyle(point)"
      >
        <span
          v-if="shouldShowAxisLabel(point.index)"
          class="trend-x-label"
          :class="{ 'is-first': point.index === 0, 'is-last': point.index === points.length - 1 }"
        >{{ point.label }}</span>
      </span>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.input-trend-chart {
  --input-chart-accent: #2f86df;
  --input-chart-accent-deep: #2574c7;
  --input-chart-point-fill: #f7f9fb;
  --chart-motion-duration: 600ms;
  --chart-motion-ease: cubic-bezier(.22, 1, .36, 1);
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr);
  grid-template-rows: minmax(224px, 1fr) 28px;
  min-height: 270px;
}

.input-trend-chart.is-range-expanding { --chart-motion-duration: 680ms; }
.input-trend-chart.is-range-contracting { --chart-motion-duration: 560ms; }

.trend-y-axis {
  grid-row: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 10px 10px 18px 0;
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
  overflow: hidden;
  isolation: isolate;
}

.trend-plot svg,
.trend-bar-layer,
.trend-value-layer,
.trend-hit-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.trend-plot svg {
  z-index: 1;
  overflow: hidden;
}

.trend-grid-line {
  stroke: color-mix(in srgb, var(--border-strong) 48%, transparent);
  stroke-width: .2;
  stroke-dasharray: 1.35 1.35;
  vector-effect: non-scaling-stroke;
}

.trend-area {
  fill: url(#input-trend-area);
}

.trend-line {
  fill: none;
  stroke: var(--input-chart-accent);
  stroke-width: 2.6;
  stroke-linecap: round;
  stroke-linejoin: round;
  vector-effect: non-scaling-stroke;
}

.is-dense .trend-line { stroke-width: 2.25; }

.line-layer-enter-active .trend-line,
.line-layer-appear-active .trend-line {
  animation: reveal-line var(--chart-motion-duration) var(--chart-motion-ease) both;
}

.line-layer-enter-active .trend-area,
.line-layer-appear-active .trend-area {
  animation: reveal-area var(--chart-motion-duration) var(--chart-motion-ease) both;
}

.line-layer-leave-active {
  transition: opacity 180ms ease-in, transform 240ms ease-in;
  transform-origin: center bottom;
}

.line-layer-leave-to {
  opacity: 0;
  transform: scaleY(.97);
}

.trend-bar-layer {
  z-index: 2;
  overflow: hidden;
  pointer-events: none;
}

.trend-value-layer {
  z-index: 3;
  overflow: hidden;
  pointer-events: none;
}

.trend-hit-layer {
  z-index: 4;
  pointer-events: none;
}

.trend-bar-node,
.trend-value-node,
.trend-hit-node,
.trend-x-node {
  position: absolute;
  inset: 0;
  transition:
    transform var(--chart-motion-duration) var(--chart-motion-ease),
    opacity 180ms ease;
  will-change: transform;
}

.trend-bar-node {
  opacity: 0;
}

.trend-bar {
  position: absolute;
  left: 0;
  bottom: 11.538%;
  width: 4.8%;
  height: 76.923%;
  border-radius: 6px 6px 2px 2px;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--input-chart-accent) 96%, white) 0%,
      color-mix(in srgb, var(--input-chart-accent-deep) 88%, var(--bg-card)) 100%
    );
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 22%, transparent);
  opacity: .92;
  transform: translateX(-50%) scale(var(--bar-scale-x), .018);
  transform-origin: center bottom;
  transition:
    transform var(--chart-motion-duration) var(--chart-motion-ease),
    opacity 180ms ease,
    background-color 180ms ease;
  will-change: transform;
}

.is-bar .trend-bar-node { opacity: 1; }
.is-bar .trend-bar {
  opacity: 1;
  transform: translateX(-50%) scale(var(--bar-scale-x), var(--bar-scale-y));
  transition-delay: var(--datum-delay);
}

.trend-bar-node.is-zero .trend-bar {
  opacity: 0;
}

.trend-bar-node.is-active .trend-bar {
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--input-chart-accent) 86%, white) 0%,
      var(--input-chart-accent) 100%
    );
}

.trend-value-node {
  opacity: 0;
}

.is-bar .trend-value-node {
  opacity: 1;
  transition-delay: calc(var(--datum-delay) + 120ms);
}

.trend-bar-value {
  position: absolute;
  left: 0;
  top: 0;
  padding: 2px 4px;
  transform: translate(-50%, calc(-100% - 7px));
  border-radius: 4px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-card) 86%, transparent);
  font: 650 10px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.is-dense .trend-bar-value {
  padding-inline: 2px;
  font-size: 9px;
}

.trend-point {
  position: absolute;
  left: 0;
  top: 0;
  width: 22px;
  height: 22px;
  padding: 0;
  transform: translate(-50%, -50%);
  border: 0;
  border-radius: 50%;
  background: transparent;
  cursor: crosshair;
  pointer-events: auto;
}

.trend-point::after {
  content: '';
  position: absolute;
  inset: 5px;
  border: 2px solid var(--input-chart-accent);
  border-radius: inherit;
  background: var(--input-chart-point-fill);
  box-shadow: 0 0 0 .5px color-mix(in srgb, var(--input-chart-accent) 64%, transparent);
  opacity: 0;
  transform: scale(.72);
  transition: opacity 160ms ease, transform 200ms var(--chart-motion-ease);
}

.is-dense .trend-point::after {
  inset: 7px;
  border-width: 1.5px;
}

.trend-point.is-marker-visible::after,
.trend-point:hover::after,
.trend-point:focus-visible::after,
.trend-point.is-active::after {
  opacity: 1;
  transform: scale(1);
}

.trend-point:hover::after,
.trend-point:focus-visible::after,
.trend-point.is-active::after {
  transform: scale(1.18);
}

.is-bar .trend-point {
  width: max(22px, calc(var(--bar-width) * 1% + 10px));
  height: max(18px, calc(var(--bar-hit-height) * 1%));
  transform: translate(-50%, 0);
  border-radius: 7px;
  cursor: pointer;
}

.trend-point:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

.trend-tooltip {
  position: absolute;
  z-index: 5;
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

.trend-x-label {
  position: absolute;
  left: 0;
  top: 9px;
  transform: translateX(-50%);
  white-space: nowrap;
}

.trend-x-label.is-first { transform: translateX(-28%); }
.trend-x-label.is-last { transform: translateX(-72%); }

.datum-enter-active,
.datum-leave-active {
  transition:
    transform var(--chart-motion-duration) var(--chart-motion-ease),
    opacity 180ms ease;
}

.datum-enter-from,
.datum-leave-to {
  opacity: 0;
}

.datum-leave-active {
  position: absolute;
}

.tooltip-enter-active,
.tooltip-leave-active { transition: opacity 120ms ease, transform 160ms var(--chart-motion-ease); }
.tooltip-enter-from,
.tooltip-leave-to { opacity: 0; transform: translate(-50%, calc(-100% - 4px)); }

@keyframes reveal-line {
  from { clip-path: inset(0 100% 0 0); opacity: .35; }
  to { clip-path: inset(0 0 0 0); opacity: 1; }
}

@keyframes reveal-area {
  from { clip-path: inset(0 100% 0 0); opacity: 0; }
  30% { opacity: .35; }
  to { clip-path: inset(0 0 0 0); opacity: 1; }
}

@media (max-width: 760px) {
  .input-trend-chart {
    grid-template-columns: 40px minmax(0, 1fr);
    grid-template-rows: minmax(202px, 1fr) 26px;
    min-height: 238px;
  }

  .trend-bar-value { font-size: 9px; }
  .is-dense .trend-bar-value { font-size: 8px; }
}

@media (prefers-reduced-motion: reduce) {
  .line-layer-enter-active .trend-line,
  .line-layer-appear-active .trend-line,
  .line-layer-enter-active .trend-area,
  .line-layer-appear-active .trend-area {
    animation: none;
  }

  .line-layer-enter-active,
  .line-layer-leave-active,
  .trend-bar-node,
  .trend-value-node,
  .trend-hit-node,
  .trend-x-node,
  .trend-bar,
  .datum-enter-active,
  .datum-leave-active,
  .tooltip-enter-active,
  .tooltip-leave-active {
    transition-duration: 1ms;
    transition-delay: 0ms;
  }
}
</style>
