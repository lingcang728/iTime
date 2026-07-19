<script setup lang="ts">
import { PhCheck, PhLockSimple, PhMedal, PhPulse, PhRobot, PhSquaresFour } from '@phosphor-icons/vue'
import type { Component } from 'vue'
import type { WeeklyAchievement } from './weeklyModel'

defineProps<{ achievements: WeeklyAchievement[] }>()
const icons: Record<string, Component> = { focus: PhMedal, rhythm: PhPulse, ai: PhRobot, apps: PhSquaresFour }

function statusLabel(item: WeeklyAchievement): string {
  if (!item.available) return '等待数据'
  return item.unlocked ? '已达成' : `${Math.round(item.progress * 100)}%`
}
</script>

<template>
  <div class="achievements">
    <article v-for="item in achievements" :key="item.id" :class="{ unlocked: item.unlocked, unavailable: !item.available }">
      <span class="achievements__badge"><component :is="icons[item.id]" :size="25" weight="regular" /><i><PhCheck v-if="item.unlocked" :size="11" weight="bold" /><PhLockSimple v-else :size="10" /></i></span>
      <span class="achievements__copy"><strong>{{ item.title }}</strong><small>{{ item.detail }}</small></span>
      <em>{{ statusLabel(item) }}</em>
      <i><span :style="{ width: `${item.progress * 100}%` }"></span></i>
    </article>
  </div>
</template>

<style scoped>
.achievements { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px; margin-top: 12px; }
.achievements article { min-width: 0; min-height: 126px; display: grid; grid-template-columns: 78px minmax(0, 1fr) auto; align-items: center; gap: 12px; padding: 12px 14px; border: 1px solid var(--border-soft); border-radius: var(--radius-lg); background: linear-gradient(135deg,var(--bg-card),var(--bg-subtle)); }
.achievements__badge { width: 68px; aspect-ratio: 1; position: relative; display: grid; place-items: center; color: var(--accent-strong); border: 2px solid color-mix(in srgb,var(--accent) 55%,var(--border-soft)); border-radius: 22px; background: var(--accent-soft); transform: rotate(45deg); }
.achievements__badge > svg { transform: rotate(-45deg); }
.achievements__badge i { width: 19px; height: 19px; position: absolute; right: -6px; bottom: -6px; display: grid; place-items: center; border-radius: 50%; color: var(--text-inverse); background: var(--accent-strong); transform: rotate(-45deg); }
.achievements__copy { min-width: 0; display: grid; gap: 2px; }
.achievements__copy strong { color: var(--text-primary); font-size: var(--text-body); font-weight: 680; }
.achievements__copy small { color: var(--text-muted); font-size: var(--text-xs); line-height: 1.45; }
.achievements em { color: var(--text-muted); font-size: 10px; font-style: normal; white-space: nowrap; }
.achievements article > i { height: 3px; grid-column: 2 / 4; align-self: start; margin-top: -18px; overflow: hidden; border-radius: 3px; background: var(--border-soft); }
.achievements article > i span { display: block; height: 100%; border-radius: inherit; background: var(--neutral-series); }
.achievements article.unlocked em { color: color-mix(in srgb, var(--accent-green) 62%, var(--text-primary)); }
.achievements article.unlocked i span { background: var(--accent-green); }
.achievements article.unavailable { opacity: .68; }
@media (max-width: 1100px) { .achievements { grid-template-columns: repeat(2,minmax(0,1fr)); } }
@media (max-width: 620px) { .achievements { grid-template-columns: 1fr; } }
</style>
