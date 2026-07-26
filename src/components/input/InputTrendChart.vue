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
const modeMotion = ref(false)
const numberFormatter = new Intl.NumberFormat('zh-CN')
const maximumBarWidth = 4.8
let previousPointCount = props.points.length
let rangeMotionTimer: number | null = null
let modeMotionTimer: number | null = null

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
const columnHitWidth = computed(() => {
  if (props.points.length <= 1) return 12
  const spacing = (100 - horizontalInset.value * 2) / Math.max(1, props.points.length - 1)
  return Math.min(14, Math.max(barWidth.value + 1.2, spacing * .92))
})
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
  if (!point) return { opacity: '0', pointerEvents: 'none' as const }
  // Anchor tooltip above the datum (line marker / bar top), clamped horizontally.
  return {
    opacity: '1',
    left: `${Math.min(92, Math.max(8, point.x))}%`,
    top: `${Math.max(8, point.y / 52 * 100 - 2)}%`,
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
  }, 420)
})

watch(() => props.mode, (mode, previousMode) => {
  hoveredIndex.value = null
  focusedIndex.value = null
  modeMotion.value = true
  if (modeMotionTimer !== null) window.clearTimeout(modeMotionTimer)
  modeMotionTimer = window.setTimeout(() => {
    modeMotion.value = false
    modeMotionTimer = null
  }, 620)
  if (mode === 'line' && previousMode !== 'line') lineAnimationEpoch.value += 1
})

onBeforeUnmount(() => {
  if (rangeMotionTimer !== null) window.clearTimeout(rangeMotionTimer)
  if (modeMotionTimer !== null) window.clearTimeout(modeMotionTimer)
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
  }
}

function datumPointStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    transform: `translate3d(${point.x}%, ${point.y / 52 * 100}%, 0)`,
  }
}

function datumBarStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    ...datumXStyle(point),
    '--bar-scale-x': String(barWidth.value / maximumBarWidth),
    '--bar-scale-y': String(point.barScale),
  }
}

/** Full-height column hit target for bar mode — snappy hover, no short-bar miss. */
function datumColumnStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return {
    transform: `translate3d(${point.x}%, 0, 0)`,
    '--column-width': String(columnHitWidth.value),
  }
}

function hitNodeStyle(point: typeof chartPoints.value[number]): Record<string, string> {
  return props.mode === 'bar' ? datumColumnStyle(point) : datumPointStyle(point)
}

function pointAriaLabel(index: number): string {
  const point = props.points[index]
  return `${point?.accessibleLabel ?? `第 ${index + 1} 天`}，${numberFormatter.format(point?.value ?? 0)} 次字符键按下`
}

function setHovered(index: number) {
  if (hoveredIndex.value !== index) hoveredIndex.value = index
}

function clearHovered() {
  hoveredIndex.value = null
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
        'is-mode-motion': modeMotion,
      },
    ]"
    :data-range-motion="rangeMotion"
    role="group"
    :aria-label="ariaLabel"
    @mouseleave="clearHovered"
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
        <Transition name="line-layer" :duration="{ enter: 380, leave: 280 }" appear>
          <g v-if="mode === 'line'" :key="lineAnimationEpoch" class="trend-line-layer">
            <path :d="areaPath" class="trend-area" />
            <path :d="linePath" class="trend-line" />
          </g>
        </Transition>
      </svg>

      <!-- Bars: no TransitionGroup FLIP — range changes use opacity + short scale only -->
      <div class="trend-bar-layer" aria-hidden="true">
        <div
          v-for="point in chartPoints"
          :key="point.accessibleLabel"
          class="trend-bar-node"
          :class="{ 'is-zero': point.value === 0, 'is-active': activeIndex === point.index }"
          :style="datumBarStyle(point)"
        >
          <span class="trend-bar" />
        </div>
      </div>

      <!-- Values: opacity-only transitions; positions snap (no floating numbers) -->
      <div class="trend-value-layer" aria-hidden="true">
        <div
          v-for="point in chartPoints"
          v-show="point.value > 0"
          :key="point.accessibleLabel"
          class="trend-value-node"
          :class="{ 'is-active': activeIndex === point.index }"
          :style="datumPointStyle(point)"
        >
          <span class="trend-bar-value">{{ formatBarValue(point.value) }}</span>
        </div>
      </div>

      <!-- Hit targets: full-height columns in bar mode for instant cursor switching -->
      <div class="trend-hit-layer">
        <div
          v-for="point in chartPoints"
          :key="point.accessibleLabel"
          class="trend-hit-node"
          :class="{ 'is-column': mode === 'bar' }"
          :data-datum-key="point.accessibleLabel"
          :style="hitNodeStyle(point)"
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
            @pointerenter="setHovered(point.index)"
            @pointermove="setHovered(point.index)"
          />
        </div>
      </div>

      <!-- Tooltip stays mounted; only opacity/position change (no leave/enter thrash) -->
      <span
        class="trend-tooltip"
        role="tooltip"
        :class="{ 'is-visible': !!activePoint }"
        :style="tooltipStyle"
        :aria-hidden="!activePoint"
      >{{ activeTooltip }}</span>
    </div>

    <div class="trend-x-axis" aria-hidden="true">
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
    </div>
  </div>
</template>

<style scoped>
.input-trend-chart {
  --input-chart-accent: #2f86df;
  --input-chart-accent-deep: #2574c7;
  --input-chart-point-fill: #f7f9fb;
  --chart-motion-duration: 360ms;
  --chart-motion-ease: cubic-bezier(.22, 1, .36, 1);
  --chart-bar-duration: 320ms;
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr);
  grid-template-rows: minmax(224px, 1fr) 28px;
  min-height: 270px;
}

.input-trend-chart.is-range-expanding,
.input-trend-chart.is-range-contracting {
  --chart-motion-duration: 340ms;
  --chart-bar-duration: 300ms;
}

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

/* Line reveal: short, no scale morph */
.line-layer-enter-active .trend-line,
.line-layer-appear-active .trend-line {
  animation: reveal-line var(--chart-motion-duration) var(--chart-motion-ease) both;
}

.line-layer-enter-active .trend-area,
.line-layer-appear-active .trend-area {
  animation: reveal-area var(--chart-motion-duration) var(--chart-motion-ease) both;
}

/* Line leave: match bar enter duration so both finish together, no gap */
.line-layer-leave-active {
  transition: opacity 280ms ease-in, clip-path 280ms var(--chart-motion-ease);
}

.line-layer-leave-to {
  opacity: 0;
  clip-path: inset(0 100% 0 0);
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
}

/* Bars: only animate height scale + brief x reflow during range change */
.trend-bar-node {
  opacity: 0;
  transition:
    opacity 160ms ease,
    transform var(--chart-bar-duration) var(--chart-motion-ease);
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
    transform var(--chart-bar-duration) var(--chart-motion-ease),
    opacity 160ms ease,
    filter 80ms ease,
    box-shadow 80ms ease;
}

.is-bar .trend-bar-node { opacity: 1; }
.is-bar .trend-bar {
  opacity: 1;
  transform: translateX(-50%) scale(var(--bar-scale-x), var(--bar-scale-y));
}

.trend-bar-node.is-zero .trend-bar {
  opacity: 0;
}

/* Active bar: brighten the bar itself — always covers full bar height, even short ones */
.trend-bar-node.is-active .trend-bar {
  filter: brightness(1.12) saturate(1.06);
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 36%, transparent),
    0 0 0 1.5px color-mix(in srgb, white 72%, var(--input-chart-accent)),
    0 4px 14px color-mix(in srgb, var(--input-chart-accent) 28%, transparent);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--input-chart-accent) 78%, white) 0%,
      var(--input-chart-accent) 100%
    );
}

/* Values: opacity only — never morph transform (kills floating numbers) */
.trend-value-node {
  opacity: 0;
  transition: opacity 160ms ease;
}

.is-bar .trend-value-node {
  opacity: 1;
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
  transition: color 80ms ease, font-weight 80ms ease;
}

.trend-value-node.is-active .trend-bar-value {
  color: var(--text-primary);
  font-weight: 700;
}

.is-dense .trend-bar-value {
  padding-inline: 2px;
  font-size: 9px;
}

/* During range change: directional clip-path reveal + position snap */
.is-range-expanding .trend-bar-layer,
.is-range-contracting .trend-bar-layer {
  animation: none; /* reset any stale animation */
}

/* Expanding (7→30d): new content clips in from the left side */
.is-range-expanding .trend-bar-layer {
  animation: range-expand-in var(--chart-bar-duration) var(--chart-motion-ease) both;
}

/* Contracting (30→7d): new content clips in from the right side */
.is-range-contracting .trend-bar-layer {
  animation: range-contract-in var(--chart-bar-duration) var(--chart-motion-ease) both;
}

@keyframes range-expand-in {
  from { clip-path: inset(0 100% 0 0); opacity: .5; }
  to   { clip-path: inset(0 0% 0 0);   opacity: 1; }
}

@keyframes range-contract-in {
  from { clip-path: inset(0 0 0 100%); opacity: .5; }
  to   { clip-path: inset(0 0% 0 0%);  opacity: 1; }
}

.is-range-expanding .trend-bar-node,
.is-range-contracting .trend-bar-node,
.is-range-expanding .trend-value-node,
.is-range-contracting .trend-value-node,
.is-range-expanding .trend-hit-node,
.is-range-contracting .trend-hit-node,
.is-range-expanding .trend-x-node,
.is-range-contracting .trend-x-node {
  transition:
    opacity 180ms ease,
    transform 280ms var(--chart-motion-ease);
}

.is-range-expanding .trend-value-node,
.is-range-contracting .trend-value-node {
  transition: opacity 140ms ease;
}

.is-range-expanding .trend-bar,
.is-range-contracting .trend-bar {
  transition:
    transform 280ms var(--chart-motion-ease),
    opacity 140ms ease;
}

/* Mode switch: coordinated, no stagger */
.is-mode-motion .trend-bar {
  transition:
    transform 600ms var(--chart-motion-ease),
    opacity 180ms ease,
    filter 80ms ease,
    box-shadow 80ms ease;
}

.is-mode-motion .trend-value-node {
  transition-duration: 200ms;
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
  touch-action: manipulation;
}

/* Line markers only — never show white circle in bar mode */
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
  transition: opacity 120ms ease, transform 160ms var(--chart-motion-ease);
  pointer-events: none;
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

/* Bar mode: kill circular marker completely; full-height column hit target */
.is-bar .trend-point::after {
  content: none;
  display: none;
}

.is-bar .trend-point {
  top: 0;
  width: calc(var(--column-width, 6) * 1%);
  height: 100%;
  min-width: 18px;
  transform: translateX(-50%);
  border-radius: 0;
  cursor: pointer;
}

.trend-point:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

/* Tooltip: stay in DOM, only fade/position — no Transition leave thrash when scrubbing bars */
.trend-tooltip {
  position: absolute;
  z-index: 5;
  max-width: 200px;
  padding: 7px 9px;
  transform: translate(-50%, calc(-100% - 12px));
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-elevated) 96%, transparent);
  box-shadow: var(--shadow-popover);
  font: 600 11px/1.3 var(--font-data);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transition:
    opacity 90ms ease,
    left 100ms linear,
    top 100ms linear;
}

.trend-tooltip.is-visible {
  opacity: 1;
}

.trend-x-axis {
  position: relative;
  grid-column: 2;
  grid-row: 2;
  color: var(--text-muted);
  font: 500 11px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.trend-x-node {
  transition: transform var(--chart-bar-duration) var(--chart-motion-ease);
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

@keyframes reveal-line {
  from { clip-path: inset(0 100% 0 0); opacity: .4; }
  to { clip-path: inset(0 0 0 0); opacity: 1; }
}

@keyframes reveal-area {
  from { clip-path: inset(0 100% 0 0); opacity: 0; }
  28% { opacity: .3; }
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

  .is-range-expanding .trend-bar-layer,
  .is-range-contracting .trend-bar-layer {
    animation: none !important;
  }

  .line-layer-enter-active,
  .line-layer-leave-active,
  .trend-bar-node,
  .trend-value-node,
  .trend-hit-node,
  .trend-x-node,
  .trend-bar,
  .trend-tooltip {
    transition-duration: 1ms !important;
  }

  .trend-tooltip {
    transition: none;
  }
}
</style>
