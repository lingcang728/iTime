<script setup lang="ts">
import { PhInfo } from '@phosphor-icons/vue'
import type { Component } from 'vue'

interface MetricValuePart {
  amount: string
  unit?: string
}

withDefaults(defineProps<{
  label: string
  value?: string
  valueParts?: readonly MetricValuePart[]
  detail: string
  icon?: Component
  info?: string
  tone?: 'neutral' | 'accent' | 'warning' | 'danger'
  visual?: 'bars' | 'ring'
}>(), { tone: 'neutral' })
</script>

<template>
  <article class="metric-card" :class="{ 'metric-card--with-icon': icon }" :data-tone="tone">
    <span v-if="icon" class="metric-icon"><component :is="icon" :size="24" weight="regular" /></span>
    <div class="metric-card__body">
      <div class="metric-card__header">
        <span>{{ label }}</span>
        <button v-if="info" class="metric-info" type="button" :aria-label="`${label}说明：${info}`">
          <PhInfo :size="13" weight="regular" />
          <span role="tooltip">{{ info }}</span>
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
    <span v-if="visual === 'bars'" class="metric-card__art metric-bars" aria-hidden="true">
      <i v-for="height in [42, 66, 78, 46, 61, 96]" :key="height" :style="{ height: `${height}%` }"></i>
    </span>
    <svg v-else-if="visual === 'ring'" class="metric-card__art metric-ring" viewBox="0 0 44 44" aria-hidden="true">
      <circle cx="22" cy="22" r="17" class="metric-ring__track" />
      <circle cx="22" cy="22" r="17" class="metric-ring__value" />
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
  padding: 0;
  border: 0;
  border-radius: 50%;
  color: var(--text-muted);
  background: transparent;
  cursor: help;
}

.metric-info [role="tooltip"] {
  width: 210px;
  position: absolute;
  z-index: 8;
  top: 24px;
  left: -8px;
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
  opacity: 0;
  pointer-events: none;
  transform: translateY(-3px);
  transition: opacity 140ms ease, transform 140ms var(--ease-out);
}

.metric-info:hover [role="tooltip"],
.metric-info:focus [role="tooltip"] {
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
  stroke-dasharray: 82 107;
  stroke-linecap: round;
}

@keyframes metric-rise {
  from { opacity: .3; transform: scaleY(.36); }
  to { opacity: 1; transform: scaleY(1); }
}
</style>
