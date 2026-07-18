<script setup lang="ts">
import { PhCheck, PhLockSimple } from '@phosphor-icons/vue'
import type { WeeklyAchievement } from './weeklyModel'

defineProps<{ achievements: WeeklyAchievement[] }>()

function statusLabel(item: WeeklyAchievement): string {
  if (!item.available) return '等待数据'
  return item.unlocked ? '已达成' : `${Math.round(item.progress * 100)}%`
}
</script>

<template>
  <div class="achievements">
    <article v-for="item in achievements" :key="item.id" :class="{ unlocked: item.unlocked, unavailable: !item.available }">
      <span class="achievements__icon"><PhCheck v-if="item.unlocked" :size="15" weight="bold" /><PhLockSimple v-else :size="14" /></span>
      <span class="achievements__copy"><strong>{{ item.title }}</strong><small>{{ item.detail }}</small></span>
      <em>{{ statusLabel(item) }}</em>
      <i><span :style="{ width: `${item.progress * 100}%` }"></span></i>
    </article>
  </div>
</template>

<style scoped>
.achievements { display: grid; margin-top: 12px; }
.achievements article { min-width: 0; min-height: 46px; display: grid; grid-template-columns: 22px minmax(0, 1fr) auto; align-items: center; gap: 8px; border-bottom: 1px solid var(--border-soft); }
.achievements__icon { width: 22px; height: 22px; display: grid; place-items: center; color: var(--text-muted); }
.achievements__copy { min-width: 0; display: grid; gap: 2px; }
.achievements__copy strong { color: var(--text-primary); font-size: 10px; font-weight: 680; }
.achievements__copy small { overflow: hidden; color: var(--text-muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.achievements em { color: var(--text-muted); font-size: 10px; font-style: normal; white-space: nowrap; }
.achievements i { height: 3px; grid-column: 2 / 4; align-self: start; margin-top: -7px; overflow: hidden; border-radius: 3px; background: var(--border-soft); }
.achievements i span { display: block; height: 100%; border-radius: inherit; background: var(--neutral-series); }
.achievements article.unlocked .achievements__icon { color: var(--accent-green-strong); }
.achievements article.unlocked em { color: color-mix(in srgb, var(--accent-green) 62%, var(--text-primary)); }
.achievements article.unlocked i span { background: var(--accent-green); }
.achievements article.unavailable { opacity: .68; }
</style>
