<script setup lang="ts">
import { computed } from 'vue'
import {
  PhClock,
  PhEye,
  PhPulse,
  PhSparkle,
  PhTarget,
} from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import { coalesceRangesBy } from '../domain/intervals'
import type { ForegroundAppInterval } from '../domain/events'
import { comparisonLabel, metricDefinitions, metricInfo } from '../domain/metricDefinitions'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatClock, formatDuration } from '../utils/format'
import { isSelectedLocalDay } from './timelineModel'

interface DurationPart {
  amount: string
  unit?: string
}

const store = useAppStore()
const isToday = computed(() => isSelectedLocalDay(store.state.selectedDate, Date.now()))
const dayLabel = computed(() => isToday.value ? '今日' : '当日')
const previousLabel = computed(() => isToday.value ? '昨日' : '前一日')
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const computerDuration = computed(() => store.day.value.computerActivity.value)
const foregroundDuration = computed(() => store.day.value.foregroundActivity.value)
const foregroundEvents = computed(() => coalesceRangesBy(
  store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'),
  (event) => event.appId,
  20_000,
))
const totalAppDuration = computed(() => store.day.value.apps.reduce((total, app) => total + app.duration, 0))
const maxAppDuration = computed(() => Math.max(1, ...store.day.value.apps.map((app) => app.duration)))
const rankingRows = computed(() => [...store.day.value.apps]
  .sort((first, second) => second.duration - first.duration)
  .slice(0, 7)
  .map((app) => ({
    ...app,
    meter: app.duration / maxAppDuration.value * 100,
    share: totalAppDuration.value ? app.duration / totalAppDuration.value : 0,
  })))
const appCategories = computed(() => new Map(store.day.value.apps.map((app) => [app.appId, app.category])))
const timelineRows = computed(() => foregroundEvents.value.slice(0, 8).map((event) => ({
  ...event,
  category: appCategories.value.get(event.appId) ?? event.category,
})))
const focusRatio = computed(() => store.day.value.foregroundFocusRatio.value)
const focusPercent = computed(() => focusRatio.value === null ? null : Math.round(focusRatio.value * 100))
const switchCount = computed(() => store.day.value.foregroundSwitches.value)
const previousDay = computed(() => store.week.value.at(-2) ?? null)
const topApp = computed(() => rankingRows.value[0] ?? null)
const longestInterval = computed(() => [...foregroundEvents.value]
  .sort((first, second) => (second.end - second.start) - (first.end - first.start))[0] ?? null)
// P4: real trend arrays for MetricCard decorative bars
const computerTrend = computed(() => store.week.value.map((d) => d.computerActivity.value ?? 0))
const foregroundTrend = computed(() => store.week.value.map((d) => d.foregroundActivity.value ?? 0))
const focusTrend = computed(() => store.week.value.map((d) => d.foregroundFocusRatio.value ?? 0))
const switchTrend = computed(() => store.week.value.map((d) => d.foregroundSwitches.value ?? 0))
const computerComparison = computed(() => comparisonLabel(
  computerDuration.value,
  previousDay.value?.computerActivity.value ?? null,
  (value) => formatDuration(value, true),
  previousLabel.value,
))
const foregroundComparison = computed(() => comparisonLabel(
  foregroundDuration.value,
  previousDay.value?.foregroundActivity.value ?? null,
  (value) => formatDuration(value, true),
  previousLabel.value,
))
const focusComparison = computed(() => comparisonLabel(
  focusRatio.value,
  previousDay.value?.foregroundFocusRatio.value ?? null,
  (value) => `${Math.round(value * 100)} 个百分点`,
  previousLabel.value,
))
const switchComparison = computed(() => comparisonLabel(
  switchCount.value,
  previousDay.value?.foregroundSwitches.value ?? null,
  (value) => `${Math.round(value)} 次`,
  previousLabel.value,
))

// 空态按数据源状态分流：降级/失败要如实呈现，不能统一回落到"等待第一条"。
const rankingEmptyTitle = computed(() => {
  const status = store.state.activityDataStatus
  if (status === 'loading') return '正在读取活动记录'
  if (status === 'degraded') return '活动记录部分可用'
  if (status === 'error') return '活动记录读取失败'
  return '等待第一条应用活动'
})
const rankingEmptyDetail = computed(() => {
  if (store.state.activityDataStatus === 'degraded') {
    return store.state.activityDataMessage
      ? `${store.state.activityDataMessage}；已有记录仍会展示`
      : '部分记录可能缺失，已有记录仍会展示'
  }
  return activityDataAvailable.value
    ? 'iTime 已开始记录，新活动会自动出现在这里。'
    : store.state.activityDataMessage
})
const activityEmptyTitle = computed(() => rankingEmptyTitle.value === '等待第一条应用活动'
  ? `${dayLabel.value}暂无活动区间`
  : rankingEmptyTitle.value)
const reminderVisible = computed(() => store.state.currentReminder !== null)

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  const minutes = Math.max(0, Math.round(value / 60_000))
  const hours = Math.floor(minutes / 60)
  const remainder = minutes % 60
  if (!hours) return [{ amount: String(remainder), unit: '分钟' }]
  return [{ amount: (minutes / 60).toFixed(1), unit: '小时' }]
}

function dismissReminder(): void {
  store.dismissCurrentReminder()
}
</script>

<template>
  <section class="page home-page">
    <PageHeader title="首页" :subtitle="`${dayLabel}概览`" />

    <div class="metrics-grid metrics-grid--home">
      <MetricCard :label="metricDefinitions.computerActivity.name" :value-parts="durationParts(computerDuration)" :detail="computerComparison" :icon="PhClock" visual="bars" :trend="computerTrend" :info="metricInfo('computerActivity')" />
      <MetricCard :label="metricDefinitions.foregroundActivity.name" :value-parts="durationParts(foregroundDuration)" :detail="foregroundComparison" :icon="PhTarget" visual="bars" :trend="foregroundTrend" :info="metricInfo('foregroundActivity')" />
      <MetricCard :label="metricDefinitions.foregroundFocusRatio.name" :value="focusPercent === null ? '—' : `${focusPercent}%`" :detail="focusComparison" :icon="PhEye" visual="ring" :progress="focusRatio" :trend="focusTrend" :info="metricInfo('foregroundFocusRatio')" />
      <MetricCard :label="metricDefinitions.foregroundSwitches.name" :value="switchCount === null ? '—' : `${switchCount} 次`" :detail="switchComparison" :icon="PhPulse" visual="bars" :trend="switchTrend" :info="metricInfo('foregroundSwitches')" />
    </div>

    <div class="home-data-grid">
      <article class="ranking-card">
        <div class="section-heading">
          <h2>应用排行</h2>
          <span class="section-meta">Top 7</span>
        </div>
        <div class="ranking-columns" aria-hidden="true"><span>应用</span><span>时长</span><span>占比</span></div>
        <div v-if="rankingRows.length" class="ranking-list">
          <div v-for="(app, index) in rankingRows" :key="app.appId" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span>
            <div class="ranking-identity">
              <ApplicationIcon :app-identity="app.appId" :app-name="app.appName" :size="22" />
              <strong>{{ app.appName }}</strong>
            </div>
            <span class="rank-duration">{{ formatDuration(app.duration, true) }}</span>
            <span class="rank-bar"><i :style="{ width: `${app.meter}%` }"></i></span>
            <span class="rank-value"><b>{{ Math.round(app.share * 100) }}%</b></span>
          </div>
        </div>
        <div v-else class="section-state" :data-state="store.state.activityDataStatus"><strong>{{ rankingEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span></div>
      </article>

      <article class="today-timeline">
        <div class="section-heading">
          <h2>{{ isToday ? '今日活动' : '当日活动' }}</h2>
          <span class="section-meta">最近 8 段</span>
        </div>
        <div v-if="timelineRows.length" class="home-activity-list" :aria-label="`${dayLabel}应用活动时间线`">
          <div v-for="event in timelineRows" :key="event.id" class="home-activity-row">
            <time :datetime="formatClock(event.start)">{{ formatClock(event.start) }}</time>
            <span class="home-activity-dot" aria-hidden="true"></span>
            <div class="home-activity-card">
              <ApplicationIcon :app-identity="event.appId" :app-name="event.appName" :size="22" />
              <span><strong>{{ event.appName }}</strong><small>{{ event.category }} · {{ event.basis }}</small></span>
              <em>{{ formatDuration(event.end - event.start, true) }}</em>
            </div>
          </div>
        </div>
        <div v-else class="section-state" :data-state="store.state.activityDataStatus"><strong>{{ activityEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span></div>
      </article>
    </div>

    <article class="home-summary-bar home-insight-card">
      <span class="insight-mark"><PhSparkle :size="22" weight="fill" aria-hidden="true" /></span>
      <div class="insight-copy">
        <strong>{{ dayLabel }}</strong>
        <p v-if="foregroundDuration !== null && focusPercent !== null">前台专注 {{ formatDuration(foregroundDuration, true) }} · 专注占比 {{ focusPercent }}%</p>
        <p v-else>数据不足，暂无结论</p>
        <small>{{ longestInterval ? `最长连续：${formatClock(longestInterval.start)}–${formatClock(longestInterval.end)}` : '继续记录后显示节奏' }}</small>
      </div>
      <div class="insight-stat"><small>最长区间</small><strong>{{ longestInterval ? `${formatClock(longestInterval.start)}–${formatClock(longestInterval.end)}` : '—' }}</strong></div>
      <div class="insight-stat"><small>最长应用</small><strong>{{ topApp?.appName ?? '—' }}<template v-if="topApp">（{{ formatDuration(topApp.duration, true) }}）</template></strong></div>
      <div class="insight-stat"><small>区间时长</small><strong>{{ longestInterval ? formatDuration(longestInterval.end - longestInterval.start, true) : '—' }}</strong></div>
      <div v-if="reminderVisible" class="wellbeing-card">
        <PhEye :size="18" aria-hidden="true" />
        <span>已连续使用 {{ store.state.currentReminder?.continuousMinutes }} 分钟</span>
        <button class="button secondary" type="button" @click="dismissReminder">知道了</button>
      </div>
    </article>
  </section>
</template>

<style scoped src="../styles/home-page.css"></style>
