<script setup lang="ts">
import ApplicationIcon from '../ApplicationIcon.vue'
import type { WeeklyApp } from './weeklyModel'
import { formatDuration } from '../../utils/format'

defineProps<{ apps: WeeklyApp[] }>()
</script>

<template>
  <ol v-if="apps.length" class="weekly-apps">
    <li v-for="(app, index) in apps" :key="app.appId">
      <span class="weekly-apps__rank">{{ String(index + 1).padStart(2, '0') }}</span>
      <ApplicationIcon :app-identity="app.appId" :app-name="app.appName" :size="28" />
      <span class="weekly-apps__name"><strong>{{ app.appName }}</strong><small>{{ app.category }}</small></span>
      <em>{{ formatDuration(app.duration, true) }}</em>
    </li>
  </ol>
  <div v-else class="weekly-apps__empty">本周尚无可验证的前台应用记录</div>
</template>

<style scoped>
.weekly-apps { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px 18px; margin: 14px 0 0; padding: 0; list-style: none; }
.weekly-apps li { min-width: 0; display: grid; grid-template-columns: 24px 30px minmax(0, 1fr) auto; align-items: center; gap: 8px; padding: 8px 10px; border: 1px solid color-mix(in srgb, var(--border-soft) 72%, transparent); border-radius: 10px; background: color-mix(in srgb, var(--bg-subtle) 68%, transparent); }
.weekly-apps__rank { color: var(--text-muted); font-size: 10px; font-variant-numeric: tabular-nums; }
.weekly-apps__name { min-width: 0; display: grid; gap: 2px; }
.weekly-apps__name strong { overflow: hidden; color: var(--text-primary); font-size: 10px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
.weekly-apps__name small { overflow: hidden; color: var(--text-muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.weekly-apps em { color: var(--text-secondary); font-size: 10px; font-style: normal; font-variant-numeric: tabular-nums; white-space: nowrap; }
.weekly-apps__empty { min-height: 110px; display: grid; place-items: center; color: var(--text-muted); font-size: 10px; }
@media (max-width: 760px) { .weekly-apps { grid-template-columns: 1fr; } }
</style>
