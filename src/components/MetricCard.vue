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
  </article>
</template>

<style scoped>
.metric-card__header {
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
</style>
