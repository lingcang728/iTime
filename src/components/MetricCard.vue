<script setup lang="ts">
import type { Component } from 'vue'

interface MetricValuePart {
  amount: string
  unit?: string
}

defineProps<{
  label: string
  value?: string
  valueParts?: readonly MetricValuePart[]
  detail: string
  icon: Component
  tone: 'blue' | 'green' | 'violet' | 'cyan' | 'orange'
}>()
</script>

<template>
  <article class="metric-card">
    <div class="metric-card__header">
      <span>{{ label }}</span>
      <span class="metric-icon" :data-tone="tone"><component :is="icon" :size="19" weight="duotone" /></span>
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
  </article>
</template>

<style scoped>
.metric-card__value {
  display: flex;
  align-items: baseline;
  gap: 7px;
  min-height: 28px;
}

.metric-value-part {
  display: inline-flex;
  align-items: baseline;
  gap: 3px;
}

.metric-value-number {
  color: var(--text-primary);
  font-family: var(--font-data);
  font-size: 25px;
  font-weight: 800;
  line-height: 1;
  letter-spacing: -0.8px;
}

.metric-value-unit {
  color: var(--text-primary);
  font-family: SimHei, "Microsoft YaHei UI", sans-serif;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0;
}
</style>
