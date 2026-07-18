<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhCaretDown,
  PhClock,
  PhEye,
  PhPulse,
  PhSparkle,
  PhTarget,
} from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import { mergeRanges } from '../domain/intervals'
import type { ForegroundAppInterval } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatClock, formatDuration } from '../utils/format'
import { shouldShowRestReminder } from '../utils/reminders'

interface DurationPart {
  amount: string
  unit?: string
}

const DISMISSED_REMINDERS_KEY = 'itime-home-dismissed-reminders-v1'
const store = useAppStore()
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const computerDuration = computed(() => store.day.value.computerActivity.value)
const foregroundDuration = computed(() => store.day.value.foregroundActivity.value)
const foregroundEvents = computed(() => store.day.value.events
  .filter((event): event is ForegroundAppInterval => event.type === 'foreground')
  .sort((first, second) => first.start - second.start))
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
const focusPercent = computed(() => computerDuration.value
  ? Math.round((foregroundDuration.value ?? 0) / computerDuration.value * 100)
  : 0)
const topApp = computed(() => rankingRows.value[0] ?? null)
const longestInterval = computed(() => [...foregroundEvents.value]
  .sort((first, second) => (second.end - second.start) - (first.end - first.start))[0] ?? null)
const rankingEmptyTitle = computed(() => activityDataAvailable.value
  ? '等待第一条应用活动'
  : store.state.activityDataStatus === 'loading' ? '正在读取活动记录' : '活动记录暂不可用')
const rankingEmptyDetail = computed(() => activityDataAvailable.value
  ? 'iTime 已开始记录，新活动会自动出现在这里。'
  : store.state.activityDataMessage)
const activeInterval = computed(() => mergeRanges(store.day.value.events
  .filter((event) => event.type === 'device' && event.state === 'active')
  .map(({ start, end }) => ({ start, end })))
  .sort((first, second) => second.end - first.end)[0])
const continuousDuration = computed(() => activeInterval.value ? activeInterval.value.end - activeInterval.value.start : 0)
const continuousTarget = computed(() => store.state.goals.continuous * 60_000)
const dismissedDates = ref<string[]>(readDismissedDates())
const reminderVisible = computed(() => shouldShowRestReminder({
  enabled: store.state.reminders,
  continuousDuration: continuousDuration.value,
  targetDuration: continuousTarget.value,
  lastActiveEnd: activeInterval.value?.end ?? null,
  now: new Date(),
  quietStart: store.state.quietStart,
  quietEnd: store.state.quietEnd,
  dismissed: dismissedDates.value.includes(store.state.selectedDate),
}))

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  const minutes = Math.max(0, Math.round(value / 60_000))
  const hours = Math.floor(minutes / 60)
  const remainder = minutes % 60
  if (!hours) return [{ amount: String(remainder), unit: '分钟' }]
  return [{ amount: (minutes / 60).toFixed(1), unit: '小时' }]
}

function readDismissedDates(): string[] {
  if (typeof localStorage === 'undefined') return []
  try {
    const value: unknown = JSON.parse(localStorage.getItem(DISMISSED_REMINDERS_KEY) ?? '[]')
    return Array.isArray(value) ? value.filter((date): date is string => typeof date === 'string') : []
  } catch {
    return []
  }
}

function dismissReminder(): void {
  dismissedDates.value = [...new Set([...dismissedDates.value, store.state.selectedDate])].slice(-30)
  localStorage.setItem(DISMISSED_REMINDERS_KEY, JSON.stringify(dismissedDates.value))
  store.showToast('今天不再显示这条休息提醒')
}
</script>

<template>
  <section class="page home-page">
    <PageHeader title="首页" subtitle="概览你的专注与时间分布，AI 帮你更好地安排每一天。" />

    <div class="metrics-grid metrics-grid--home">
      <MetricCard label="总使用时长" :value-parts="durationParts(computerDuration)" detail="较昨日  +1.2 小时" :icon="PhClock" visual="bars" />
      <MetricCard label="深度工作时长" :value-parts="durationParts(foregroundDuration)" detail="较昨日  +0.4 小时" :icon="PhTarget" visual="bars" />
      <MetricCard label="专注度" :value="`${focusPercent}%`" detail="较昨日  +6%" :icon="PhEye" visual="ring" />
      <MetricCard label="切换次数" :value="`${foregroundEvents.length} 次`" detail="较昨日  -8 次" :icon="PhPulse" visual="bars" />
    </div>

    <div class="home-data-grid">
      <article class="ranking-card">
        <div class="section-heading">
          <h2>应用使用排行</h2>
          <button class="text-button" type="button">查看全部</button>
        </div>
        <div class="ranking-columns" aria-hidden="true"><span>应用</span><span>使用时长</span><span>占比</span></div>
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
        <div v-else class="section-state"><strong>{{ rankingEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span></div>
        <button v-if="rankingRows.length" class="ranking-more" type="button">查看全部应用与网站</button>
      </article>

      <article class="today-timeline">
        <div class="section-heading">
          <h2>时间线 · 今日活动</h2>
          <button class="text-button" type="button">集中视图<PhCaretDown :size="13" weight="bold" aria-hidden="true" /></button>
        </div>
        <div v-if="timelineRows.length" class="home-activity-list" aria-label="今日应用活动时间线">
          <div v-for="event in timelineRows" :key="event.id" class="home-activity-row">
            <time>{{ formatClock(event.start) }}</time>
            <span class="home-activity-dot" aria-hidden="true"></span>
            <div class="home-activity-card">
              <ApplicationIcon :app-identity="event.appId" :app-name="event.appName" :size="22" />
              <span><strong>{{ event.appName }}</strong><small>{{ event.category }} · {{ event.basis }}</small></span>
              <em>{{ formatDuration(event.end - event.start, true) }}</em>
            </div>
          </div>
        </div>
        <div v-else class="section-state"><strong>{{ rankingEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span></div>
      </article>
    </div>

    <article class="home-summary-bar home-insight-card">
      <span class="insight-mark"><PhSparkle :size="22" weight="fill" /></span>
      <div class="insight-copy">
        <strong>今日洞察</strong>
        <p>你的深度工作时长为 {{ formatDuration(foregroundDuration ?? 0, true) }}，专注度达到 {{ focusPercent }}%。</p>
        <small>{{ longestInterval ? `${formatClock(longestInterval.start)}–${formatClock(longestInterval.end)} 是今天最长的连续工作区间。` : '记录更多活动后，这里会给出更准确的工作节奏建议。' }}</small>
      </div>
      <div class="insight-stat"><small>高效时段</small><strong>{{ longestInterval ? `${formatClock(longestInterval.start)}–${formatClock(longestInterval.end)}` : '—' }}</strong></div>
      <div class="insight-stat"><small>专注时长最长应用</small><strong>{{ topApp?.appName ?? '—' }}<template v-if="topApp">（{{ formatDuration(topApp.duration, true) }}）</template></strong></div>
      <div class="insight-stat"><small>最佳专注时长</small><strong>{{ longestInterval ? formatDuration(longestInterval.end - longestInterval.start, true) : '—' }}</strong></div>
      <div v-if="reminderVisible" class="wellbeing-card">
        <PhEye :size="18" />
        <span>已连续使用 {{ formatDuration(continuousDuration, true) }}</span>
        <button class="button secondary" type="button" @click="dismissReminder">知道了</button>
      </div>
    </article>
  </section>
</template>

<style scoped src="../styles/home-page.css"></style>
