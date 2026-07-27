<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import { PhInfo } from '@phosphor-icons/vue'
import type { Component, CSSProperties } from 'vue'

interface MetricValuePart {
  amount: string
  unit?: string
}

const props = withDefaults(defineProps<{
  label: string
  value?: string
  valueParts?: readonly MetricValuePart[]
  detail: string
  icon?: Component
  info?: string
  tone?: 'neutral' | 'accent' | 'warning' | 'danger'
  visual?: 'bars' | 'ring'
  trend?: readonly number[]
  progress?: number | null
}>(), { tone: 'neutral' })

const trendHeights = computed<number[]>(() => {
  if (!props.trend?.length) return []
  const max = Math.max(1, ...props.trend)
  return props.trend.map((v) => Math.max(8, Math.round(v / max * 96)))
})
const normalizedProgress = computed(() => {
  if (props.progress === null || props.progress === undefined || !Number.isFinite(props.progress)) return null
  return Math.max(0, Math.min(1, props.progress))
})
const ringOffset = computed(() => normalizedProgress.value === null ? 107 : 107 * (1 - normalizedProgress.value))

const infoButtonRef = ref<HTMLButtonElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const tooltipOpen = ref(false)
const tooltipStyle = ref<CSSProperties>({
  top: '0px',
  left: '0px',
  width: '240px',
})

const TOOLTIP_WIDTH = 240
const VIEWPORT_GAP = 8
const ANCHOR_GAP = 8

function placeTooltip(): void {
  const button = infoButtonRef.value
  if (!button) return

  const rect = button.getBoundingClientRect()
  const width = Math.min(TOOLTIP_WIDTH, Math.max(160, window.innerWidth - VIEWPORT_GAP * 2))
  let left = rect.left + rect.width / 2 - width / 2
  left = Math.min(Math.max(VIEWPORT_GAP, left), window.innerWidth - width - VIEWPORT_GAP)

  const measuredHeight = tooltipRef.value?.offsetHeight || 0
  const estimatedHeight = measuredHeight > 0 ? measuredHeight : 112
  const spaceBelow = window.innerHeight - rect.bottom - VIEWPORT_GAP
  const spaceAbove = rect.top - VIEWPORT_GAP
  const placeAbove = spaceBelow < estimatedHeight + ANCHOR_GAP && spaceAbove > spaceBelow

  let top = placeAbove
    ? rect.top - ANCHOR_GAP - estimatedHeight
    : rect.bottom + ANCHOR_GAP
  top = Math.min(
    Math.max(VIEWPORT_GAP, top),
    Math.max(VIEWPORT_GAP, window.innerHeight - estimatedHeight - VIEWPORT_GAP),
  )

  tooltipStyle.value = {
    top: `${Math.round(top)}px`,
    left: `${Math.round(left)}px`,
    width: `${Math.round(width)}px`,
  }
}

function openTooltip(): void {
  tooltipOpen.value = true
  placeTooltip()
  void nextTick(() => {
    placeTooltip()
  })
  window.addEventListener('scroll', placeTooltip, true)
  window.addEventListener('resize', placeTooltip)
}

function closeTooltip(): void {
  tooltipOpen.value = false
  window.removeEventListener('scroll', placeTooltip, true)
  window.removeEventListener('resize', placeTooltip)
}

onBeforeUnmount(() => {
  window.removeEventListener('scroll', placeTooltip, true)
  window.removeEventListener('resize', placeTooltip)
})
</script>

<template>
  <article
    class="metric-card"
    :class="{
      'metric-card--with-icon': icon,
      'metric-card--tooltip-open': tooltipOpen,
    }"
    :data-tone="tone"
  >
    <span v-if="icon" class="metric-icon"><component :is="icon" :size="24" weight="regular" /></span>
    <div class="metric-card__body">
      <div class="metric-card__header">
        <span>{{ label }}</span>
        <button
          v-if="info"
          ref="infoButtonRef"
          class="metric-info"
          type="button"
          :class="{ 'is-open': tooltipOpen }"
          :aria-label="`${label}说明：${info}`"
          :aria-expanded="tooltipOpen"
          @mouseenter="openTooltip"
          @mouseleave="closeTooltip"
          @focus="openTooltip"
          @blur="closeTooltip"
          @keydown.escape.prevent="closeTooltip"
        >
          <PhInfo :size="13" weight="regular" />
          <span
            ref="tooltipRef"
            class="metric-info__tooltip"
            role="tooltip"
            :class="{ 'is-open': tooltipOpen }"
            :style="tooltipStyle"
          >{{ info }}</span>
        </button>
      </div>
      <strong class="metric-card__value">
        <template v-if="valueParts?.length">
          <span v-for="(part, index) in valueParts" :key="`${part.amount}-${index}`" class="metric-value-part">
            <span class="metric-value-number">{{ part.amount }}</span>
            <span v-if="part.unit" class="metric-value-unit">{{ part.unit }}</span>
          </span>
        </template>
        <span v-else class="metric-value-number">{{ value }}</span>
      </strong>
      <small>{{ detail }}</small>
    </div>
    <span v-if="visual === 'bars' && trendHeights.length" class="metric-card__art metric-bars" aria-hidden="true">
      <i v-for="(height, idx) in trendHeights" :key="idx" :style="{ height: `${height}%` }"></i>
    </span>
    <svg v-else-if="visual === 'ring' && normalizedProgress !== null" class="metric-card__art metric-ring" viewBox="0 0 44 44" aria-hidden="true">
      <circle cx="22" cy="22" r="17" class="metric-ring__track" />
      <circle cx="22" cy="22" r="17" class="metric-ring__value" :style="{ strokeDashoffset: ringOffset }" />
    </svg>
  </article>
</template>

<style scoped>
.metric-card__header {
  position: relative;
}

.metric-card {
  position: relative;
}

.metric-card--with-icon {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr);
  align-items: start;
  column-gap: 12px;
}

.metric-card--tooltip-open {
  /* Allow the open definition bubble to paint above sibling cards. */
  z-index: 6;
  overflow: visible;
}

.metric-card__body {
  min-width: 0;
}

.metric-card:has(.metric-card__art) .metric-card__body {
  padding-right: 64px;
}

.metric-card__value {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 27px;
}

.metric-value-part {
  display: inline-flex;
  align-items: baseline;
  gap: 3px;
}

.metric-value-number {
  color: var(--text-primary);
  font-family: var(--font-data);
  font-size: var(--text-metric);
  font-weight: 700;
  line-height: 1;
  letter-spacing: -0.8px;
}

.metric-card[data-tone="accent"] .metric-value-number {
  color: var(--text-primary);
}

.metric-value-unit {
  color: var(--text-secondary);
  font-family: var(--font-ui);
  font-size: var(--text-xs);
  font-weight: 600;
  letter-spacing: 0;
}

.metric-info {
  width: 18px;
  height: 18px;
  position: relative;
  display: grid;
  place-items: center;
  flex: 0 0 18px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  color: var(--text-muted);
  background: transparent;
  cursor: help;
}

.metric-info:hover,
.metric-info:focus-visible,
.metric-info.is-open {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-soft) 88%, transparent);
}

/*
 * Fixed positioning keeps the definition readable even when the metric card
 * (or a parent metrics row) uses overflow clipping for rounded shells.
 */
.metric-info__tooltip {
  position: fixed;
  z-index: 1200;
  box-sizing: border-box;
  max-width: min(240px, calc(100vw - 16px));
  padding: 9px 10px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-popover);
  font-size: var(--text-xs);
  font-weight: 500;
  line-height: 1.55;
  text-align: left;
  white-space: normal;
  overflow-wrap: anywhere;
  opacity: 0;
  pointer-events: none;
  transform: translateY(-3px);
  transition: opacity 140ms ease, transform 140ms var(--ease-out);
}

.metric-info__tooltip.is-open,
.metric-info:hover .metric-info__tooltip,
.metric-info:focus .metric-info__tooltip,
.metric-info:focus-visible .metric-info__tooltip {
  opacity: 1;
  transform: translateY(0);
}

.metric-card__art {
  position: absolute;
  right: 18px;
  bottom: 22px;
  width: 56px;
  height: 50px;
}

.metric-bars {
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  gap: 4px;
}

.metric-bars i {
  width: 5px;
  min-height: 8px;
  border-radius: 3px;
  background: color-mix(in srgb, var(--accent) 48%, var(--bg-soft));
  transform-origin: bottom;
  animation: metric-rise 360ms var(--ease-out) both;
}

.metric-bars i:nth-child(2) { animation-delay: 30ms; }
.metric-bars i:nth-child(3) { animation-delay: 60ms; }
.metric-bars i:nth-child(4) { animation-delay: 90ms; }
.metric-bars i:nth-child(5) { animation-delay: 120ms; }
.metric-bars i:nth-child(6) { animation-delay: 150ms; }

.metric-ring {
  overflow: visible;
  transform: rotate(-90deg);
}

.metric-ring circle {
  fill: none;
  stroke-width: 4;
}

.metric-ring__track {
  stroke: var(--bg-soft);
}

.metric-ring__value {
  stroke: var(--accent-strong);
  stroke-dasharray: 107;
  stroke-linecap: round;
  transition: stroke-dashoffset 260ms var(--ease-out);
}

@keyframes metric-rise {
  from { opacity: .3; transform: scaleY(.36); }
  to { opacity: 1; transform: scaleY(1); }
}
</style>
