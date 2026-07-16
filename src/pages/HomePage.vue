<script setup lang="ts">
import { computed, ref } from 'vue'
import { PhBrain, PhDesktop, PhEye, PhKeyboard, PhMicrophone, PhMoonStars, PhRobot } from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { categoryVisuals, fallbackCategoryVisual } from '../data/visualCatalog'
import { aggregateCategories } from '../domain/metrics'
import { mergeRanges } from '../domain/intervals'
import type { TimelineSegment } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatNumber } from '../utils/format'

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
  visual: categoryVisuals[category.category] ?? fallbackCategoryVisual,
  meter: category.duration / maxCategory.value * 100,
})))
const computerDuration = computed(() => store.day.value.computerActivity.value)
const foregroundDuration = computed(() => store.day.value.foregroundActivity.value)
const aiDuration = computed(() => store.day.value.aiEffective.value)
const voiceCharacters = computed(() => store.day.value.events.reduce((total, event) => event.type === 'voice' ? total + event.characters : total, 0))
const activeInterval = computed(() => mergeRanges(store.day.value.events
  .filter((event) => event.type === 'device' && event.state === 'active')
  .map(({ start, end }) => ({ start, end })))
  .sort((a, b) => b.end - a.end)[0])
const continuousDuration = computed(() => activeInterval.value ? activeInterval.value.end - activeInterval.value.start : 0)
const continuousTarget = computed(() => store.state.goals.continuous * 60_000)
const timelineSegments = computed<TimelineSegment[]>(() => store.day.value.events
  .flatMap((event): TimelineSegment[] => {
    if (event.type === 'foreground') return [{ start: event.start, end: event.end, kind: 'attention' }]
    if (event.type === 'aiWork') return [{ start: event.start, end: event.end, kind: 'agent' }]
    if (event.type === 'media') return [{ start: event.start, end: event.end, kind: 'media' }]
    return []
  })
  .sort((a, b) => a.start - b.start))
const dismissedDates = ref<string[]>(readDismissedDates())
const reminderVisible = computed(() => store.state.reminders
  && continuousDuration.value >= continuousTarget.value
  && !dismissedDates.value.includes(store.state.selectedDate))
const timeTicks = [0, 4, 8, 12, 16, 20, 24]

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
      <MetricCard label="电脑活动时间" :value-parts="durationParts(computerDuration)" :detail="store.state.activityDataMessage" :icon="PhDesktop" tone="blue" />
      <MetricCard label="主动注意力" :value-parts="durationParts(foregroundDuration)" :detail="shareOf(foregroundDuration, computerDuration)" :icon="PhBrain" tone="green" />
      <MetricCard label="AI 代理工作" :value-parts="durationParts(aiDuration)" :detail="store.day.value.aiEffective.basis" :icon="PhRobot" tone="violet" />
      <MetricCard label="语音输入" :value-parts="durationParts(store.day.value.voiceDuration.value)" :detail="`${formatNumber(voiceCharacters)} 字`" :icon="PhMicrophone" tone="cyan" />
      <MetricCard label="离座播放" :value-parts="durationParts(store.day.value.mediaDuration.value)" :detail="shareOf(store.day.value.mediaDuration.value, computerDuration)" :icon="PhMoonStars" tone="orange" />
    </div>

    <div class="home-columns">
      <article class="card ranking-card">
        <div class="section-heading">
          <div><h2>应用与分类排行</h2><p>按今天的前台活动时间统计</p></div>
          <div class="ranking-value-labels" aria-hidden="true"><span>时间</span><span>占比</span></div>
        </div>
        <div v-if="rankingRows.length" class="ranking-list">
          <div v-for="(category, index) in rankingRows" :key="category.category" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span>
            <div class="ranking-identity">
              <span class="category-icon" :style="{ color: category.visual.iconTone }"><component :is="category.visual.icon" :size="16" weight="duotone" /></span>
              <ApplicationIcon :icon-key="category.representative.appId" :app-name="category.representative.appName" :size="20" />
              <span><strong>{{ category.category }}</strong><small>{{ category.representative.appName }}</small></span>
            </div>
            <span class="rank-bar"><i :style="{ width: `${category.meter}%`, background: category.visual.gradient }"></i></span>
            <span class="rank-value" :aria-label="`${formatDuration(category.duration, true)}，占比 ${Math.round(category.share * 100)}%`">
              <strong>{{ formatDuration(category.duration, true) }}</strong><b>{{ Math.round(category.share * 100) }}%</b>
            </span>
          </div>
        </div>
        <p v-else class="home-empty">iTime 已开始记录，新的应用活动会出现在这里。</p>
      </article>
      <article class="card input-summary-card">
        <div class="input-summary-icon"><PhKeyboard :size="22" weight="duotone" /></div>
        <div><span>今日输入摘要</span><strong>{{ formatNumber(store.input.value.cumulative.keyStrokes) }} 次敲击</strong><small>{{ store.input.value.source }}</small></div>
      </article>
    </div>

    <article class="card today-timeline">
      <div class="section-heading"><div><h2>今日时间线</h2><p>把注意力、AI 与离座播放放在同一条时间轴上</p></div><div class="legend"><span class="green">主动注意力</span><span class="violet">AI 代理</span><span class="orange">离座播放</span></div></div>
      <div class="time-axis"><span v-for="tick in timeTicks" :key="tick" :style="{ left: `${tick / 24 * 100}%` }">{{ String(tick).padStart(2, '0') }}:00</span></div>
      <TimelineLane v-if="timelineSegments.length" :range="store.day.value.range" :segments="timelineSegments" />
      <p v-else class="timeline-empty">等待第一段本机活动记录</p>
    </article>

    <Transition name="page">
      <article v-if="reminderVisible" class="wellbeing-card">
        <div class="wellbeing-icon"><PhEye :size="24" weight="duotone" /></div>
        <div><span>温润提醒</span><h2>已连续使用 {{ formatDuration(continuousDuration, true) }}{{ continuousDuration >= continuousTarget ? '，建议休息眼睛' : '' }}</h2><p>每隔 {{ store.state.goals.continuous }} 分钟，远眺 20 秒，让眼睛放松一下。</p></div>
        <button class="button secondary" type="button" @click="dismissReminder">知道了</button>
      </article>
    </Transition>
  </section>
</template>

<style scoped src="../styles/home-page.css"></style>
