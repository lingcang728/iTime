<script setup lang="ts">
import { computed, type Component } from 'vue'
import { PhInfo } from '@phosphor-icons/vue'

const props = defineProps<{
  label: string
  value: string
  detail: string
  icon: Component
  tone: 'blue' | 'green' | 'violet' | 'cyan' | 'orange'
  info?: string
}>()

const tokens = computed(() => props.value.split(/(\d+(?:\.\d+)?)/).filter(Boolean).map((text) => ({
  text,
  numeric: /^\d+(?:\.\d+)?$/.test(text),
})))
</script>

<template>
  <article class="agent-metric-card">
    <div class="agent-metric-card__top">
      <span>{{ label }}</span>
      <button v-if="info" class="metric-info" type="button" :aria-label="`${label}说明`">
        <PhInfo :size="15" weight="bold" />
        <span role="tooltip">{{ info }}</span>
      </button>
    </div>
    <span class="agent-metric-icon" :data-tone="tone"><component :is="icon" :size="25" weight="duotone" /></span>
    <strong class="agent-metric-value"><span v-for="(token, index) in tokens" :key="index" :class="{ number: token.numeric }">{{ token.text }}</span></strong>
    <small>{{ detail }}</small>
  </article>
</template>
