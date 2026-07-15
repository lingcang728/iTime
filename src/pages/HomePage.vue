<script setup lang="ts">
import { computed } from 'vue'
import { PhBrain, PhDesktop, PhKeyboard, PhMicrophone, PhMoonStars, PhRobot, PhEye } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import MetricCard from '../components/MetricCard.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatNumber } from '../utils/format'
import type { ForegroundAppInterval } from '../domain/events'

const store = useAppStore()
const maxApp = computed(() => Math.max(1, ...store.day.value.apps.map((app) => app.duration)))
const foregroundSegments = computed(() => store.day.value.events
  .filter((event): event is ForegroundAppInterval => event.type === 'foreground')
  .map((event) => ({ start: event.start, end: event.end, color: event.color })))
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
        <div class="section-heading"><div><h2>应用与分类排行</h2><p>按前台活动时间</p></div><button class="text-button" type="button">按分类</button></div>
        <div class="ranking-list">
          <div v-for="(app, index) in store.day.value.apps" :key="app.appId" class="ranking-row">
            <span class="rank">{{ index + 1 }}</span><span class="app-dot" :style="{ background: app.color }"></span><strong>{{ app.appName }}</strong>
            <span class="rank-bar"><i :style="{ width: `${app.duration / maxApp * 100}%`, background: app.color }"></i></span>
            <span>{{ formatDuration(app.duration, true) }}</span>
          </div>
        </div>
      </article>
      <article class="card input-summary-card">
        <div class="input-summary-icon"><PhKeyboard :size="22" weight="duotone" /></div>
        <div><span>今日输入摘要</span><strong>{{ formatNumber(store.input.value.cumulative.keyStrokes) }} 次敲击</strong><small>输入密度最高时段 10:00—11:00</small></div>
      </article>
    </div>
    <article class="card today-timeline">
      <div class="section-heading"><div><h2>今日时间线</h2><p>00:00—24:00</p></div><div class="legend"><span class="green">前台活动</span><span class="violet">AI 代理</span><span class="orange">媒体播放</span></div></div>
      <div class="time-axis"><span>00:00</span><span>04:00</span><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span><span>24:00</span></div>
      <TimelineLane :range="store.day.value.range" :segments="foregroundSegments" />
    </article>
    <article class="wellbeing-card">
      <div class="wellbeing-icon"><PhEye :size="24" weight="duotone" /></div>
      <div><span>温润提醒</span><h2>已连续使用 42 分钟，建议休息眼睛</h2><p>每隔 20 分钟，远眺 20 秒，让眼睛放松一下吧。</p></div>
      <button class="button secondary" type="button">知道了</button>
    </article>
  </section>
</template>

