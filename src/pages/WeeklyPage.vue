<script setup lang="ts">
import { computed } from 'vue'
import { PhChartLineUp, PhClock, PhKeyboard, PhRobot, PhSquaresFour, PhTarget } from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import WeeklyAchievements from '../components/weekly/WeeklyAchievements.vue'
import WeeklyTopApps from '../components/weekly/WeeklyTopApps.vue'
import WeeklyTrendChart from '../components/weekly/WeeklyTrendChart.vue'
import { buildWeeklySummary } from '../components/weekly/weeklyModel'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatNumber } from '../utils/format'

interface DurationPart { amount: string; unit?: string }

const store = useAppStore()
const hour = 3_600_000
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const summary = computed(() => buildWeeklySummary(store.week.value))
const trendPoints = computed(() => summary.value.days.map((day) => ({
  label: day.label, note: day.note,
  attention: day.foreground === null ? null : day.foreground / hour,
  ai: day.providerExecution === null ? null : day.providerExecution / hour,
})))
const recordedDays = computed(() => summary.value.days.filter((day) => day.computer !== null).length)
const inputDays = computed(() => summary.value.days.filter((day) => day.input !== null).length)
const calendarMaximum = computed(() => Math.max(1, ...summary.value.days.map((day) => day.foreground ?? 0)))

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  const minutes = Math.round(value / 60_000)
  return [{ amount: String(Math.floor(minutes / 60)), unit: '小时' }, { amount: String(minutes % 60), unit: '分' }]
}

function averageDuration(total: number | null, days: number): string {
  return total === null || !days ? '无日均基线' : `日均 ${formatDuration(total / days, true)}`
}

function calendarLevel(value: number | null): number {
  return value ? Math.max(1, Math.min(5, Math.ceil(value / calendarMaximum.value * 5))) : 0
}
</script>

<template>
  <section class="page weekly-page">
    <PageHeader title="周报" subtitle="回顾过去一周，持续精进每一天。" :range-label="summary.rangeLabel" />

    <div v-if="!activityDataAvailable" class="section-state weekly-source-state"><strong>{{ store.state.activityDataStatus === 'loading' ? '正在读取本周活动记录' : '本周活动记录暂不可用' }}</strong><span>{{ store.state.activityDataMessage }}</span></div>
    <template v-else>
      <div class="weekly-metrics">
        <MetricCard label="深度工作" :value-parts="durationParts(summary.totalAttention)" :detail="averageDuration(summary.totalAttention, recordedDays)" :icon="PhTarget" tone="accent" visual="bars" />
        <MetricCard label="Provider 执行" :value-parts="durationParts(summary.totalProviderExecution)" :detail="summary.totalProviderCoverage === null ? '本周没有 Provider 证据' : `自然时间覆盖 ${formatDuration(summary.totalProviderCoverage, true)}`" :icon="PhRobot" visual="bars" />
        <MetricCard label="总使用时长" :value-parts="durationParts(summary.totalComputer)" :detail="averageDuration(summary.totalComputer, recordedDays)" :icon="PhClock" visual="bars" />
        <MetricCard label="总输入字数" :value="summary.totalInput === null ? '—' : formatNumber(summary.totalInput)" :detail="inputDays ? `覆盖 ${inputDays} 个有记录日` : '本周没有输入记录'" :icon="PhKeyboard" visual="bars" />
      </div>

      <section class="weekly-achievements card"><div class="section-heading"><div><h2>本周成就</h2><p>只用深度工作、节奏、Provider 与应用数据解锁</p></div></div><WeeklyAchievements :achievements="summary.achievements" /></section>

      <div class="weekly-dashboard">
        <section class="weekly-trend card"><div class="section-heading"><div><h2>本周趋势</h2><p>深度工作与 Provider 累计执行（小时）</p></div></div><WeeklyTrendChart :points="trendPoints" secondary-label="Provider 执行" /></section>

        <section class="weekly-highlights card">
          <div class="section-heading"><div><h2>本周亮点</h2><p>全部来自可回溯的周汇总</p></div></div>
          <div class="highlight-list">
            <article><span><PhTarget :size="20" /></span><div><strong>深度工作峰值</strong><small>{{ summary.bestDay ? `${summary.bestDay.label} · ${formatDuration(summary.bestDay.foreground, true)}` : '暂无记录' }}</small></div></article>
            <article><span><PhRobot :size="20" /></span><div><strong>AI 协作证据</strong><small>{{ summary.totalProviderExecution === null ? '本周未连接 Provider' : `累计执行 ${formatDuration(summary.totalProviderExecution, true)}` }}</small></div></article>
            <article><span><PhChartLineUp :size="20" /></span><div><strong>输入峰值</strong><small>{{ summary.peakInputDay ? `${summary.peakInputDay.label} · ${formatNumber(summary.peakInputDay.input ?? 0)} 字` : '暂无输入记录' }}</small></div></article>
            <article><span><PhSquaresFour :size="20" /></span><div><strong>应用多样性</strong><small>本周使用 {{ summary.topApps.length }} 个有记录应用</small></div></article>
          </div>
        </section>

        <div class="weekly-side-stack">
          <section class="weekly-apps-card card"><div class="section-heading"><div><h2>应用 / 活动排行</h2><p>按深度工作前台时长</p></div></div><WeeklyTopApps :apps="summary.topApps.slice(0, 5)" /></section>
          <section class="weekly-calendar card"><div class="section-heading"><div><h2>本周日历热力</h2><p>深度工作 · 小时</p></div><span>低　高</span></div>
            <div class="calendar-days"><button v-for="day in summary.days" :key="day.date" type="button" :data-heat="calendarLevel(day.foreground)" :aria-label="`${day.label} ${day.note}，深度工作 ${formatDuration(day.foreground, true)}`"><span>{{ day.label }}</span><small>{{ day.note }}</small><strong>{{ day.foreground === null ? '—' : (day.foreground / hour).toFixed(1) }}</strong></button></div>
          </section>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped src="./weekly-page.css"></style>
