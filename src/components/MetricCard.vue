<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{
  label: string
  value: string
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
    <strong class="metric-card__value">{{ value }}</strong>
    <small>{{ detail }}</small>
  </article>
</template>

