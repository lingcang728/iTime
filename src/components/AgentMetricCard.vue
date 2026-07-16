<script setup lang="ts">
import { computed } from 'vue'
import { PhInfo } from '@phosphor-icons/vue'

const props = defineProps<{
  label: string
  value: string
  detail: string
  iconSrc: string
  tone: 'blue' | 'violet' | 'cyan' | 'orange'
  info: string
}>()

const tokens = computed(() => props.value.split(/(\d+(?:\.\d+)?)/).filter(Boolean).map((text) => ({
  text,
  numeric: /^\d+(?:\.\d+)?$/.test(text),
})))
</script>

<template>
  <article class="metric" :data-tone="tone">
    <div class="metric__copy">
      <div class="metric__label">
        <span>{{ label }}</span>
        <button class="metric__info" type="button" :aria-label="`${label}说明：${info}`">
          <PhInfo :size="14" weight="bold" />
          <span role="tooltip">{{ info }}</span>
        </button>
      </div>
      <strong class="metric__value">
        <span v-for="(token, index) in tokens" :key="index" :class="{ number: token.numeric }">{{ token.text }}</span>
      </strong>
      <small>{{ detail }}</small>
    </div>
    <span class="metric__art" aria-hidden="true"><img :src="iconSrc" alt="" /></span>
  </article>
</template>

<style scoped>
.metric {
  min-width: 0;
  min-height: 118px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 15px 14px 14px 16px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
}

.metric__copy { min-width: 0; }
.metric__label { display: flex; align-items: center; gap: 5px; color: var(--text-secondary); font-size: 9px; }

.metric__info {
  width: 19px;
  height: 19px;
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

.metric__info:focus-visible { outline: 2px solid var(--accent-blue); outline-offset: 1px; }
.metric__info [role="tooltip"] {
  width: 190px;
  position: absolute;
  z-index: 8;
  top: 23px;
  left: -8px;
  padding: 8px 10px;
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  color: var(--text-primary);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
  font-size: 9px;
  font-weight: 500;
  line-height: 1.55;
  text-align: left;
  opacity: 0;
  pointer-events: none;
  transform: translateY(-3px);
  transition: opacity 140ms ease, transform 140ms ease;
}
.metric__info:hover [role="tooltip"], .metric__info:focus [role="tooltip"] { opacity: 1; transform: translateY(0); }

.metric__value { display: block; margin-top: 12px; font-size: 12px; font-weight: 500; letter-spacing: -.3px; }
.metric__value .number { font-size: 25px; font-weight: 760; font-variant-numeric: tabular-nums; }
.metric small { display: block; margin-top: 7px; color: var(--text-muted); font-size: 8px; }

.metric__art {
  width: 58px;
  height: 58px;
  flex: 0 0 58px;
  display: grid;
  place-items: center;
  border-radius: 17px;
  background: color-mix(in srgb, var(--accent-violet-soft) 48%, var(--bg-soft));
}
.metric[data-tone="blue"] .metric__art { background: color-mix(in srgb, var(--accent-blue-soft) 58%, var(--bg-soft)); }
.metric[data-tone="cyan"] .metric__art { background: color-mix(in srgb, var(--accent-cyan-soft) 58%, var(--bg-soft)); }
.metric[data-tone="orange"] .metric__art { background: color-mix(in srgb, var(--accent-orange-soft) 58%, var(--bg-soft)); }
.metric__art img { width: 54px; height: 54px; object-fit: contain; filter: drop-shadow(0 8px 9px rgba(34, 38, 45, .08)); }

@media (max-width: 980px) {
  .metric { min-height: 104px; padding: 11px 10px 11px 12px; }
  .metric__art { width: 48px; height: 48px; flex-basis: 48px; }
  .metric__art img { width: 46px; height: 46px; }
  .metric__value { margin-top: 8px; }
  .metric__value .number { font-size: 21px; }
}
</style>
