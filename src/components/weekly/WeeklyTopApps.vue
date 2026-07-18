<script setup lang="ts">
import { computed } from 'vue'
import ApplicationIcon from '../ApplicationIcon.vue'
import type { WeeklyApp } from './weeklyModel'
import { formatDuration } from '../../utils/format'

const props = defineProps<{ apps: WeeklyApp[] }>()
const maximum = computed(() => props.apps.reduce((current, app) => Math.max(current, app.duration), 1))
</script>

<template>
  <ol v-if="apps.length" class="weekly-apps">
    <li v-for="(app, index) in apps" :key="app.appId">
      <span class="weekly-apps__rank">{{ String(index + 1).padStart(2, '0') }}</span>
      <ApplicationIcon :app-identity="app.appId" :app-name="app.appName" :size="26" />
      <span class="weekly-apps__name"><strong>{{ app.appName }}</strong><small>{{ app.category }}</small></span>
      <i class="weekly-apps__track" aria-hidden="true"><span :style="{ width: `${app.duration / maximum * 100}%` }"></span></i>
      <em>{{ formatDuration(app.duration, true) }}</em>
    </li>
  </ol>
  <div v-else class="weekly-apps__empty">本周尚无可验证的前台应用记录</div>
</template>

<style scoped>
.weekly-apps { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 28px; margin: 14px 0 0; padding: 0; list-style: none; }
.weekly-apps li { min-width: 0; min-height: 46px; display: grid; grid-template-columns: 18px 26px minmax(84px, 1fr) minmax(56px, .8fr) auto; align-items: center; gap: 9px; border-bottom: 1px solid var(--border-soft); }
.weekly-apps__rank { color: var(--text-muted); font-size: 10px; font-variant-numeric: tabular-nums; }
.weekly-apps__name { min-width: 0; display: grid; gap: 2px; }
.weekly-apps__name strong { overflow: hidden; color: var(--text-primary); font-size: 11px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
.weekly-apps__name small { overflow: hidden; color: var(--text-muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.weekly-apps__track { height: 4px; overflow: hidden; border-radius: 99px; background: var(--bg-soft); }
.weekly-apps__track span { display: block; height: 100%; border-radius: inherit; background: var(--accent-green); }
.weekly-apps em { color: var(--text-secondary); font-size: 10px; font-style: normal; font-variant-numeric: tabular-nums; white-space: nowrap; }
.weekly-apps__empty { min-height: 110px; display: grid; place-items: center; color: var(--text-muted); font-size: 10px; }
@media (max-width: 760px) {
  .weekly-apps { grid-template-columns: 1fr; }
  .weekly-apps li { grid-template-columns: 18px 26px minmax(84px, 1fr) minmax(52px, .65fr) auto; }
}
</style>
