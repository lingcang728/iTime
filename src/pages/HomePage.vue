<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhDesktop,
  PhEye,
  PhKeyboard,
  PhPlayCircle,
  PhRobot,
  PhTarget,
} from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import HomeTimelineBands from '../components/HomeTimelineBands.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import { aggregateCategories } from '../domain/metrics'
import { mergeRanges } from '../domain/intervals'
import type { TimelineSegment } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { hasActivityData, hasInputData } from '../stores/dataAvailability'
import { formatDuration, formatNumber } from '../utils/format'
import { shouldShowRestReminder } from '../utils/reminders'

interface DurationPart {
  amount: string
  unit?: string
}

const DISMISSED_REMINDERS_KEY = 'itime-home-dismissed-reminders-v1'
const store = useAppStore()
const categories = computed(() => aggregateCategories(store.day.value.apps))
const maxCategory = computed(() => Math.max(1, ...categories.value.map((category) => category.duration)))
const rankingRows = computed(() => categories.value.map((category) => ({
  ...category,
  meter: category.duration / maxCategory.value * 100,
})))
const computerDuration = computed(() => store.day.value.computerActivity.value)
const foregroundDuration = computed(() => store.day.value.foregroundActivity.value)
const aiDuration = computed(() => store.day.value.aiInteraction.value)
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const inputDataAvailable = computed(() => hasInputData(store.state.inputDataStatus))
const rankingEmptyTitle = computed(() => activityDataAvailable.value ? '等待第一条应用活动' : store.state.activityDataStatus === 'loading' ? '正在读取活动记录' : '活动记录暂不可用')
const rankingEmptyDetail = computed(() => activityDataAvailable.value ? 'iTime 已开始记录，新活动会自动出现在这里。' : store.state.activityDataMessage)
const inputSummaryDetail = computed(() => inputDataAvailable.value ? '只呈现聚合计数，不保存输入内容' : store.state.inputDataMessage)
const activeInterval = computed(() => mergeRanges(store.day.value.events
  .filter((event) => event.type === 'device' && event.state === 'active')
  .map(({ start, end }) => ({ start, end })))
  .sort((a, b) => b.end - a.end)[0])
const continuousDuration = computed(() => activeInterval.value ? activeInterval.value.end - activeInterval.value.start : 0)
const continuousTarget = computed(() => store.state.goals.continuous * 60_000)
const timelineSegments = computed<TimelineSegment[]>(() => store.day.value.events
  .flatMap((event): TimelineSegment[] => {
    if (event.type === 'foreground') return [{ start: event.start, end: event.end, kind: 'attention' }]
    if (event.type === 'aiInteraction') return [{ start: event.start, end: event.end, kind: 'interaction', variant: 'outline' }]
    if (event.type === 'media') return [{ start: event.start, end: event.end, kind: 'media' }]
    return []
  })
  .sort((a, b) => a.start - b.start))
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
  if (!remainder) return [{ amount: String(hours), unit: '小时' }]
  return [{ amount: String(hours), unit: '小时' }, { amount: String(remainder), unit: '分钟' }]
}

function shareOf(value: number | null, total: number | null): string {
  if (value === null || !total) return '等待活动记录'
  return `占电脑活动 ${Math.round(value / total * 100)}%`
}

function inputSummaryValue(value: number): string {
  return inputDataAvailable.value ? formatNumber(value) : '—'
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
    <PageHeader title="今天的数字生活" subtitle="真实记录，帮助你看清时间去向" />

    <div class="metrics-grid metrics-grid--home">
      <MetricCard label="电脑活动时间" :value-parts="durationParts(computerDuration)" :detail="store.state.activityDataMessage" :icon="PhDesktop" />
      <MetricCard label="主动注意力" :value-parts="durationParts(foregroundDuration)" :detail="shareOf(foregroundDuration, computerDuration)" :icon="PhTarget" tone="accent" />
      <MetricCard label="AI 前台活跃" :value-parts="durationParts(aiDuration)" :detail="store.day.value.aiInteraction.basis" :icon="PhRobot" />
      <MetricCard label="离座播放" :value-parts="durationParts(store.day.value.mediaDuration.value)" :detail="shareOf(store.day.value.mediaDuration.value, computerDuration)" :icon="PhPlayCircle" />
    </div>

    <div class="home-data-grid">
      <article class="ranking-card">
        <div class="section-heading">
          <div class="section-heading__title"><div><h2>应用与分类排行</h2><p>按今天的前台活动时间统计</p></div></div>
          <div class="ranking-value-labels" aria-hidden="true"><span>时间</span><span>占比</span></div>
        </div>
        <div v-if="rankingRows.length" class="ranking-list">
          <div v-for="(category, index) in rankingRows" :key="category.category" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span>
            <div class="ranking-identity">
              <ApplicationIcon :icon-key="category.representative.appId" :app-name="category.representative.appName" :size="20" />
              <span><strong>{{ category.category }}</strong><small>{{ category.representative.appName }}</small></span>
            </div>
            <span class="rank-bar"><i :style="{ width: `${category.meter}%` }"></i></span>
            <span class="rank-value" :aria-label="`${formatDuration(category.duration, true)}，占比 ${Math.round(category.share * 100)}%`">
              <strong>{{ formatDuration(category.duration, true) }}</strong><b>{{ Math.round(category.share * 100) }}%</b>
            </span>
          </div>
        </div>
        <div v-if="rankingRows.length" class="ranking-total">
          <span>总计（按前台）</span>
          <strong>{{ formatDuration(foregroundDuration ?? 0, true) }}</strong>
          <b>100%</b>
        </div>
        <div v-else class="section-state"><strong>{{ rankingEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span></div>
      </article>

      <article class="today-timeline">
        <div class="section-heading">
          <div class="section-heading__title"><div><h2>今日时间线</h2><p>主动注意力、AI 前台与离座播放在同一条时间轴上</p></div></div>
          <div class="legend"><span class="attention">主动注意力</span><span class="interaction">AI 前台</span><span class="media">离座播放</span></div>
        </div>
        <HomeTimelineBands v-if="activityDataAvailable" :range="store.day.value.range" :segments="timelineSegments" />
        <div v-else class="section-state home-timeline-state" :data-state="store.state.activityDataStatus">
          <strong>{{ rankingEmptyTitle }}</strong><span>{{ rankingEmptyDetail }}</span>
        </div>
      </article>
    </div>

    <article class="home-summary-bar">
      <div class="input-summary-icon"><PhKeyboard :size="21" weight="regular" /></div>
      <div class="input-summary-copy"><span>今日输入摘要</span><strong>{{ inputSummaryValue(store.input.value.cumulative.keyStrokes) }}{{ inputDataAvailable ? ' 次敲击' : '' }}</strong><small>{{ inputSummaryDetail }}</small></div>
      <div class="home-summary-end">
        <Transition name="page" mode="out-in">
          <div v-if="reminderVisible" key="reminder" class="wellbeing-card">
            <div class="wellbeing-icon"><PhEye :size="20" weight="regular" /></div>
            <div><span>休息提醒</span><strong>已连续使用 {{ formatDuration(continuousDuration, true) }}</strong></div>
            <button class="button secondary" type="button" @click="dismissReminder">知道了</button>
          </div>
          <div v-else key="stats" class="input-summary-stats">
            <span><small>鼠标点击</small><strong>{{ inputSummaryValue(store.input.value.cumulative.combinedClicks) }}</strong></span>
            <span><small>滚动</small><strong>{{ inputSummaryValue(store.input.value.cumulative.scrollDistance) }}</strong></span>
          </div>
        </Transition>
      </div>
    </article>
  </section>
</template>

<style scoped src="../styles/home-page.css"></style>
