<script setup lang="ts">
import { computed, ref } from 'vue'
import { PhClock, PhEye, PhPulse, PhRobot, PhTarget } from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { buildHomeComposition, countApplicationSwitches, dayTimelineSegments, relativeChange } from '../domain/dashboardModel'
import type { ForegroundAppInterval, TimelineKind } from '../domain/events'
import { mergeRanges } from '../domain/intervals'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatClock, formatDuration } from '../utils/format'
import { shouldShowRestReminder } from '../utils/reminders'

interface DurationPart { amount: string; unit?: string }

const DISMISSED_REMINDERS_KEY = 'itime-home-dismissed-reminders-v1'
const store = useAppStore()
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const computerDuration = computed(() => store.day.value.computerActivity.value)
const deepWorkDuration = computed(() => store.day.value.foregroundActivity.value)
const previousComputerDuration = computed(() => store.previousDay.value.computerActivity.value)
const previousDeepWorkDuration = computed(() => store.previousDay.value.foregroundActivity.value)
const foregroundEvents = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const previousForegroundEvents = computed(() => store.previousDay.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const switches = computed(() => countApplicationSwitches(foregroundEvents.value))
const previousSwitches = computed(() => countApplicationSwitches(previousForegroundEvents.value))
const focusPercent = computed(() => computerDuration.value ? Math.round((deepWorkDuration.value ?? 0) / computerDuration.value * 100) : 0)
const previousFocusPercent = computed(() => previousComputerDuration.value ? Math.round((previousDeepWorkDuration.value ?? 0) / previousComputerDuration.value * 100) : 0)
const composition = computed(() => buildHomeComposition(store.day.value))
const compositionTotal = computed(() => Math.max(1, composition.value.total ?? 0))
const compositionRows = computed(() => [
  { label: 'AI 前台', duration: composition.value.aiForeground, color: 'accent', share: composition.value.aiForeground / compositionTotal.value },
  { label: '其他深度工作', duration: composition.value.otherDeepWork, color: 'dark', share: composition.value.otherDeepWork / compositionTotal.value },
  { label: '非前台使用', duration: composition.value.nonForeground, color: 'soft', share: composition.value.nonForeground / compositionTotal.value },
])
const ringStyle = computed(() => {
  const ai = compositionRows.value[0].share * 360
  const deep = ai + compositionRows.value[1].share * 360
  return { '--ring-ai': `${ai}deg`, '--ring-deep': `${deep}deg` }
})
const timeline = computed(() => dayTimelineSegments(store.day.value))
const timelineRows = computed(() => [
  { label: '电脑活动', kind: 'other' as TimelineKind, segments: timeline.value.overview },
  { label: '深度工作', kind: 'attention' as TimelineKind, segments: timeline.value.deepWork },
  { label: 'AI 前台', kind: 'agent' as TimelineKind, segments: timeline.value.aiForeground },
  { label: '非前台使用', kind: 'media' as TimelineKind, segments: timeline.value.nonForeground },
])
const totalAppDuration = computed(() => store.day.value.apps.reduce((total, app) => total + app.duration, 0))
const maxAppDuration = computed(() => Math.max(1, ...store.day.value.apps.map((app) => app.duration)))
const rankingRows = computed(() => store.day.value.apps.slice(0, 6).map((app) => ({
  ...app,
  meter: app.duration / maxAppDuration.value * 100,
  share: totalAppDuration.value ? app.duration / totalAppDuration.value : 0,
})))
const rankingEmptyTitle = computed(() => activityDataAvailable.value ? '等待第一条应用活动' : store.state.activityDataStatus === 'loading' ? '正在读取活动记录' : '活动记录暂不可用')
const activeInterval = computed(() => mergeRanges(store.day.value.events
  .filter((event) => event.type === 'device' && event.state === 'active')
  .map(({ start, end }) => ({ start, end })))
  .sort((first, second) => second.end - first.end)[0])
const continuousDuration = computed(() => activeInterval.value ? activeInterval.value.end - activeInterval.value.start : 0)
const dismissedDates = ref<string[]>(readDismissedDates())
const reminderVisible = computed(() => shouldShowRestReminder({
  enabled: store.state.reminders,
  continuousDuration: continuousDuration.value,
  targetDuration: store.state.goals.continuous * 60_000,
  lastActiveEnd: activeInterval.value?.end ?? null,
  now: new Date(), quietStart: store.state.quietStart, quietEnd: store.state.quietEnd,
  dismissed: dismissedDates.value.includes(store.state.selectedDate),
}))

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  const minutes = Math.max(0, Math.round(value / 60_000))
  return minutes >= 60
    ? [{ amount: String(Math.floor(minutes / 60)), unit: '小时' }, { amount: String(minutes % 60), unit: '分' }]
    : [{ amount: String(minutes), unit: '分钟' }]
}

function comparisonDetail(current: number | null, previous: number | null, suffix = ''): string {
  const change = relativeChange(current, previous)
  if (change === null) return '昨日暂无可比数据'
  const sign = change > 0 ? '+' : ''
  return `较昨日 ${sign}${Math.round(change * 100)}%${suffix}`
}

function focusComparison(): string {
  if (!previousComputerDuration.value) return '昨日暂无可比数据'
  const delta = focusPercent.value - previousFocusPercent.value
  return `较昨日 ${delta > 0 ? '+' : ''}${delta} 个百分点`
}

function readDismissedDates(): string[] {
  if (typeof localStorage === 'undefined') return []
  try {
    const value: unknown = JSON.parse(localStorage.getItem(DISMISSED_REMINDERS_KEY) ?? '[]')
    return Array.isArray(value) ? value.filter((date): date is string => typeof date === 'string') : []
  } catch { return [] }
}

function dismissReminder(): void {
  dismissedDates.value = [...new Set([...dismissedDates.value, store.state.selectedDate])].slice(-30)
  localStorage.setItem(DISMISSED_REMINDERS_KEY, JSON.stringify(dismissedDates.value))
  store.showToast('今天不再显示这条休息提醒')
}
</script>

<template>
  <section class="page home-page">
    <PageHeader title="专注让时间更有价值。" subtitle="用真实活动记录回顾今天，也看见昨天没有说完的变化。" />

    <div class="metrics-grid metrics-grid--home">
      <MetricCard label="总使用时长" :value-parts="durationParts(computerDuration)" :detail="comparisonDetail(computerDuration, previousComputerDuration)" :icon="PhClock" visual="bars" />
      <MetricCard label="深度工作时长" :value-parts="durationParts(deepWorkDuration)" :detail="comparisonDetail(deepWorkDuration, previousDeepWorkDuration)" :icon="PhTarget" tone="accent" visual="bars" />
      <MetricCard label="专注度" :value="`${focusPercent}%`" :detail="focusComparison()" :icon="PhEye" visual="ring" />
      <MetricCard label="切换次数" :value="`${switches} 次`" :detail="comparisonDetail(switches, previousSwitches)" :icon="PhPulse" visual="bars" info="相邻前台应用发生变化时计为一次切换；同一应用的连续区间不会重复计数。" />
    </div>

    <article class="composition-card card">
      <div class="section-heading"><div><h2>时间构成</h2><p>三个分区互不重叠，合计为电脑总使用时长</p></div></div>
      <div v-if="composition.total !== null" class="composition-content">
        <div class="composition-ring" :style="ringStyle" role="img" :aria-label="`AI 前台 ${Math.round(compositionRows[0].share * 100)}%，其他深度工作 ${Math.round(compositionRows[1].share * 100)}%，非前台使用 ${Math.round(compositionRows[2].share * 100)}%`">
          <span><strong>{{ formatDuration(composition.total, true) }}</strong><small>电脑活动</small></span>
        </div>
        <div class="composition-list">
          <div v-for="row in compositionRows" :key="row.label" :data-color="row.color">
            <i aria-hidden="true"></i><span><strong>{{ row.label }}</strong><small>{{ Math.round(row.share * 100) }}%</small></span><b>{{ formatDuration(row.duration, true) }}</b>
          </div>
        </div>
        <div class="composition-note"><PhRobot :size="23" /><span><strong>证据边界</strong><small>AI 前台来自可识别的前台交互；Provider 后台执行在“AI 代理”中单独展示，不混入本环。</small></span></div>
      </div>
      <div v-else class="section-state"><strong>{{ rankingEmptyTitle }}</strong><span>{{ store.state.activityDataMessage }}</span></div>
    </article>

    <div class="home-bottom-grid">
      <article class="ranking-card card">
        <div class="section-heading"><div><h2>应用使用排行</h2><p>按真实前台时长排序</p></div></div>
        <div v-if="rankingRows.length" class="ranking-list">
          <div v-for="(app, index) in rankingRows" :key="app.appId" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span>
            <ApplicationIcon :app-identity="app.appId" :app-name="app.appName" :size="22" />
            <strong>{{ app.appName }}</strong>
            <span class="rank-bar"><i :style="{ width: `${app.meter}%` }"></i></span>
            <time>{{ formatDuration(app.duration, true) }}</time>
            <b>{{ Math.round(app.share * 100) }}%</b>
          </div>
        </div>
        <div v-else class="section-state"><strong>{{ rankingEmptyTitle }}</strong><span>{{ store.state.activityDataMessage }}</span></div>
      </article>

      <article class="day-band-card card">
        <div class="section-heading"><div><h2>24 小时时间带</h2><p>同一时轴上的电脑、深度工作与 AI 前台证据</p></div></div>
        <div class="day-band-axis" aria-hidden="true"><span v-for="tick in ['00:00','04:00','08:00','12:00','16:00','20:00','24:00']" :key="tick">{{ tick }}</span></div>
        <div class="day-band-rows">
          <div v-for="row in timelineRows" :key="row.label" class="day-band-row">
            <span>{{ row.label }}</span>
            <TimelineLane :range="store.day.value.range" :segments="row.segments.map((segment) => ({ ...segment, kind: row.kind }))" />
          </div>
        </div>
        <div v-if="reminderVisible" class="wellbeing-card"><PhEye :size="17" /><span>已连续使用 {{ formatDuration(continuousDuration, true) }}，适合休息一下。</span><button type="button" @click="dismissReminder">今天不再提醒</button></div>
      </article>
    </div>
  </section>
</template>

<style scoped src="../styles/home-page.css"></style>
