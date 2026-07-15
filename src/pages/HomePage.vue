<script setup lang="ts">
import { computed, ref } from 'vue'
import { PhBrain, PhDesktop, PhKeyboard, PhMicrophone, PhMoonStars, PhRobot, PhEye } from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import PageHeader from '../components/PageHeader.vue'
import MetricCard from '../components/MetricCard.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatNumber } from '../utils/format'
import { aggregateCategories } from '../domain/metrics'
import { categoryVisuals, fallbackCategoryVisual } from '../data/visualCatalog'
import type { TimelineSegment } from '../domain/events'

const store = useAppStore()
const rankMode = ref<'time' | 'share'>('time')
const categories = computed(() => aggregateCategories(store.day.value.apps))
const maxCategory = computed(() => Math.max(1, ...categories.value.map((category) => category.duration)))
const rankingRows = computed(() => categories.value.map((category) => ({
  ...category,
  visual: categoryVisuals[category.category] ?? fallbackCategoryVisual,
  meter: rankMode.value === 'time' ? category.duration / maxCategory.value * 100 : category.share * 100,
})))
const timelineSegments = computed<TimelineSegment[]>(() => {
  const segments: TimelineSegment[] = []
  for (const event of store.day.value.events) {
    if (event.type === 'device' && event.state === 'active') segments.push({ start: event.start, end: event.end, kind: 'other' })
  }
  for (const event of store.day.value.events) {
    if (event.type === 'foreground') segments.push({ start: event.start, end: event.end, kind: 'attention' })
    if (event.type === 'aiWork') segments.push({ start: event.start, end: event.end, kind: 'agent' })
    if (event.type === 'media') segments.push({ start: event.start, end: event.end, kind: 'media' })
  }
  return segments
})
const timeTicks = [4, 8, 12, 16, 20]
</script>

<template>
  <section class="page home-page">
    <PageHeader title="今天的数字生活" subtitle="专注生活，科技为你服务" />
    <div class="metrics-grid metrics-grid--home">
      <MetricCard label="电脑活动时间" :value="formatDuration(store.day.value.computerActivity.value, true)" detail="较昨日 -12%" :icon="PhDesktop" tone="blue" />
      <MetricCard label="前台活动时间" :value="formatDuration(store.day.value.foregroundActivity.value, true)" detail="占电脑活动 64%" :icon="PhBrain" tone="green" />
      <MetricCard label="AI 有效代理工时" :value="formatDuration(store.day.value.aiEffective.value, true)" detail="多个代理可重叠累计" :icon="PhRobot" tone="violet" />
      <MetricCard label="语音输入" :value="formatDuration(store.day.value.voiceDuration.value, true)" detail="共 1,284 字" :icon="PhMicrophone" tone="cyan" />
      <MetricCard label="离座播放" :value="formatDuration(store.day.value.mediaDuration.value, true)" detail="不计入前台活动" :icon="PhMoonStars" tone="orange" />
    </div>
    <div class="home-columns">
      <article class="card ranking-card">
        <div class="section-heading">
          <div><h2>应用与分类排行</h2><p>六类前台活动 · 代表应用</p></div>
          <div class="ranking-mode" role="group" aria-label="排行榜显示方式">
            <button type="button" :aria-pressed="rankMode === 'time'" @click="rankMode = 'time'">按时间</button>
            <button type="button" :aria-pressed="rankMode === 'share'" @click="rankMode = 'share'">按占比</button>
          </div>
        </div>
        <div class="ranking-list">
          <div v-for="(category, index) in rankingRows" :key="category.category" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span>
            <div class="ranking-identity">
              <span class="category-icon" :style="{ color: category.visual.iconTone }"><component :is="category.visual.icon" :size="16" weight="duotone" /></span>
              <ApplicationIcon :icon-key="category.representative.appId" :size="20" />
              <span><strong>{{ category.category }}</strong><small>{{ category.representative.appName }}</small></span>
            </div>
            <span class="rank-bar"><i :style="{ width: `${category.meter}%`, background: category.visual.gradient }"></i></span>
            <span class="rank-value">
              <strong>{{ rankMode === 'time' ? formatDuration(category.duration, true) : `${Math.round(category.share * 100)}%` }}</strong>
              <small>{{ rankMode === 'time' ? `${Math.round(category.share * 100)}%` : formatDuration(category.duration, true) }}</small>
            </span>
          </div>
        </div>
      </article>
      <article class="card input-summary-card">
        <div class="input-summary-icon"><PhKeyboard :size="22" weight="duotone" /></div>
        <div><span>今日输入摘要</span><strong>{{ formatNumber(store.input.value.cumulative.keyStrokes) }} 次敲击</strong><small>输入密度最高时段 10:00—11:00</small></div>
      </article>
    </div>
    <article class="card today-timeline">
      <div class="section-heading"><div><h2>今日时间线</h2><p>全天活动分布</p></div><div class="legend"><span class="green">主动注意力</span><span class="violet">AI 代理</span><span class="orange">离座 / 播放</span><span class="gray">其他</span></div></div>
      <div class="time-axis"><span v-for="tick in timeTicks" :key="tick" :style="{ left: `${tick / 24 * 100}%` }">{{ String(tick).padStart(2, '0') }}:00</span></div>
      <TimelineLane :range="store.day.value.range" :segments="timelineSegments" />
    </article>
    <article class="wellbeing-card">
      <div class="wellbeing-icon"><PhEye :size="24" weight="duotone" /></div>
      <div><span>温润提醒</span><h2>已连续使用 42 分钟，建议休息眼睛</h2><p>每隔 20 分钟，远眺 20 秒，让眼睛放松一下吧。</p></div>
      <button class="button secondary" type="button">知道了</button>
    </article>
  </section>
</template>
