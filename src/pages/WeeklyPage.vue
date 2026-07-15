<script setup lang="ts">
import { computed } from 'vue'
import { PhKeyboard, PhSparkle } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import BarChart from '../components/BarChart.vue'
import SparkLine from '../components/SparkLine.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatNumber } from '../utils/format'

const store = useAppStore()
const dayLabels = ['周四', '周五', '周六', '周日', '周一', '周二', '周三']
const activityPoints = computed(() => store.week.value.map((day, index) => ({ label: dayLabels[index], value: (day.computerActivity.value ?? 0) / 3_600_000, note: `5/${14 + index}` })))
const comparePoints = computed(() => store.week.value.map((day, index) => ({ label: dayLabels[index], value: (day.foregroundActivity.value ?? 0) / 3_600_000, secondary: (day.aiEffective.value ?? 0) / 3_600_000, note: `5/${14 + index}` })))
const topApps = computed(() => {
  const totals = new Map<string, { name: string; duration: number; color: string }>()
  store.week.value.forEach((day) => day.apps.forEach((app) => {
    const current = totals.get(app.appId)
    if (current) current.duration += app.duration
    else totals.set(app.appId, { name: app.appName, duration: app.duration, color: app.color })
  }))
  return [...totals.values()].sort((a, b) => b.duration - a.duration).slice(0, 5)
})
const focusValues = computed(() => store.week.value.map((day) => (day.foregroundActivity.value ?? 0) / 3_600_000))
const inputValues = computed(() => store.week.value.map((day) => Number(day.inputKeyStrokes.value ?? 0)))
</script>

<template>
  <section class="page weekly-page">
    <PageHeader title="本周回顾" subtitle="回顾一周，持续优化数字生活" range-label="5月14日 – 5月20日" />
    <div class="weekly-charts">
      <article class="card chart-card"><div class="section-heading"><div><h2>每日电脑活动</h2><p>小时</p></div></div><BarChart :points="activityPoints" /></article>
      <article class="card chart-card"><div class="section-heading"><div><h2>前台活动 vs AI 代理工作</h2><p>绿色为前台，紫色为代理工时</p></div></div><BarChart :points="comparePoints" compact /></article>
    </div>
    <div class="weekly-insights-grid">
      <article class="card focus-card">
        <div class="section-heading"><div><h2>专注热力图</h2><p>按小时活动强度</p></div></div>
        <div class="heatmap"><span v-for="index in 49" :key="index" :style="{ opacity: 0.18 + ((index * 7) % 9) / 12 }"></span></div>
        <div class="heatmap-scale"><span>低</span><i></i><span>高</span></div>
      </article>
      <article class="card top-apps-card"><div class="section-heading"><div><h2>Top 应用</h2><p>本周累计</p></div></div><ol><li v-for="(app, index) in topApps" :key="app.name"><span>{{ index + 1 }}</span><i :style="{ background: app.color }"></i><strong>{{ app.name }}</strong><em>{{ formatDuration(app.duration, true) }}</em></li></ol></article>
      <article class="card best-day-card"><PhSparkle :size="20" weight="duotone" /><span>本周洞察</span><h2>最专注的一天是周六</h2><strong>{{ focusValues[2].toFixed(1) }} 小时</strong><p>前台活动比上周同日增加 18%。</p><SparkLine :values="focusValues" color="#6979df" /></article>
    </div>
    <article class="input-rhythm-card">
      <div class="input-summary-icon"><PhKeyboard :size="23" weight="duotone" /></div>
      <div><span>输入节奏</span><h2>本周共有 {{ formatNumber(inputValues.reduce((total, value) => total + value, 0)) }} 次键盘敲击</h2><p>最高输入密度出现在周六上午，平均每 47 分钟出现一次明显休息间隔。</p></div>
      <div class="input-mini-chart"><SparkLine :values="inputValues" color="#56ae83" /></div>
    </article>
  </section>
</template>
