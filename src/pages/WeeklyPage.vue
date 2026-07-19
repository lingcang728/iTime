<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhChartLineUp,
  PhClock,
  PhInfo,
  PhKeyboard,
  PhMoon,
  PhStar,
  PhTarget,
  PhTrophy,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import WeeklyAchievements from '../components/weekly/WeeklyAchievements.vue'
import WeeklyTopApps from '../components/weekly/WeeklyTopApps.vue'
import WeeklyTrendChart from '../components/weekly/WeeklyTrendChart.vue'
import { buildWeeklySummary } from '../components/weekly/weeklyModel'
import type { ForegroundAppInterval } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatNumber } from '../utils/format'

const store = useAppStore()
const hour = 3_600_000
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取本周活动记录' : '本周活动记录暂不可用')
const summary = computed(() => buildWeeklySummary(store.week.value))
const trendPoints = computed(() => summary.value.days.map((day) => ({
  label: day.label,
  note: day.note,
  attention: day.foreground === null ? null : day.foreground / hour,
  ai: day.ai === null ? null : day.ai / hour,
})))
const dailyMaximum = computed(() => Math.max(1, ...summary.value.days.map((day) => day.computer ?? 0)))
const averageComputer = computed(() => {
  const values = summary.value.days.flatMap((day) => day.computer === null ? [] : [day.computer])
  return values.length ? values.reduce((total, value) => total + value, 0) / values.length : null
})
const activeInputDays = computed(() => summary.value.days.filter((day) => (day.input ?? 0) > 0).length)
const averageInput = computed(() => summary.value.totalInput === null || !activeInputDays.value
  ? null
  : summary.value.totalInput / activeInputDays.value)
const microBars = [28, 38, 47, 33, 62, 44, 77, 51, 67, 58]
const heatHours = [9, 11, 13, 15, 17, 19, 21]
const lockedHeatCell = ref<string | null>(null)
const hourlyHeat = computed(() => {
  const values = heatHours.flatMap((startHour) => store.week.value.map((day) => {
    const dayDate = new Date(day.range.start)
    dayDate.setHours(startHour, 0, 0, 0)
    const start = dayDate.getTime()
    const end = start + 2 * hour
    return day.events
      .filter((event): event is ForegroundAppInterval => event.type === 'foreground')
      .reduce((total, event) => total + Math.max(0, Math.min(end, event.end) - Math.max(start, event.start)), 0)
  }))
  const maximum = Math.max(1, ...values)
  return heatHours.map((startHour, rowIndex) => ({
    label: `${String(startHour).padStart(2, '0')}:00`,
    cells: summary.value.days.map((day, dayIndex) => {
      const value = values[rowIndex * summary.value.days.length + dayIndex] ?? 0
      return { value, intensity: value ? Math.max(1, Math.ceil(value / maximum * 5)) : 0, day }
    }),
  }))
})

function hourLabel(value: number | null | undefined): string {
  return value === null || value === undefined ? '暂无数据' : `${(value / hour).toFixed(1)} 小时`
}

function comparisonLabel(value: number | null, basis: 'previousWeek' | 'peerDays' | null): string {
  if (value === null) return '暂无上周基线'
  const reference = basis === 'peerDays' ? '本周其余有记录日平均' : '上周'
  return `与${reference}相比，深度工作时长 ${value >= 0 ? '↑' : '↓'} ${Math.abs(value)}%`
}

function heatCellId(date: string, label: string): string {
  return `${date}-${label}`
}

function moveHeatFocus(event: KeyboardEvent, index: number): void {
  const offset = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 }[event.key]
  if (offset === undefined || !(event.currentTarget instanceof HTMLButtonElement)) return
  event.preventDefault()
  const cells = event.currentTarget.closest('.weekly-heatmap')?.querySelectorAll<HTMLButtonElement>('.heat-cell')
  cells?.[Math.max(0, Math.min((cells.length ?? 1) - 1, index + offset))]?.focus()
}
</script>

<template>
  <section class="page weekly-page">
    <PageHeader title="周报" :subtitle="summary.rangeLabel" range-label="本周" />

    <div v-if="!activityDataAvailable" class="section-state weekly-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <template v-else>
      <section class="weekly-section weekly-daily-card" aria-labelledby="daily-title">
        <h2 id="daily-title">每日电脑活动</h2>
        <div class="weekly-days">
          <article v-for="day in summary.days" :key="day.date" :class="{ best: day.date === summary.bestDay?.date }">
            <span><PhStar v-if="day.date === summary.bestDay?.date" :size="12" weight="fill" />{{ day.label }}<small>{{ day.note }}</small></span>
            <strong>{{ day.computer === null ? '—' : (day.computer / hour).toFixed(1) }}<small v-if="day.computer !== null">小时</small></strong>
            <div class="daily-spark" aria-hidden="true"><i v-for="(height, index) in microBars" :key="index" :style="{ height: `${Math.max(8, height * ((day.computer ?? 0) / dailyMaximum))}%` }"></i></div>
          </article>
          <article class="average-day">
            <span>平均</span>
            <strong>{{ averageComputer === null ? '—' : (averageComputer / hour).toFixed(1) }}<small v-if="averageComputer !== null">小时</small></strong>
            <div class="daily-spark" aria-hidden="true"><i v-for="(height, index) in microBars" :key="index" :style="{ height: `${height}%` }"></i></div>
          </article>
        </div>
      </section>

      <div class="weekly-analysis-grid">
        <section class="weekly-section focus-panel">
          <header class="weekly-section-heading"><h2>专注热力图 <PhInfo :size="14" /></h2></header>
          <div class="weekly-heatmap" role="grid" aria-label="本周分时专注热力图">
            <span></span><span v-for="day in summary.days" :key="day.date" class="heat-day">{{ day.label }}<small>{{ day.note }}</small></span>
            <template v-for="(row, rowIndex) in hourlyHeat" :key="row.label">
              <span class="heat-hour">{{ row.label }}</span>
              <button
                v-for="(cell, cellIndex) in row.cells"
                :key="heatCellId(cell.day.date, row.label)"
                type="button"
                class="heat-cell"
                :class="[`intensity-${cell.intensity}`, { locked: lockedHeatCell === heatCellId(cell.day.date, row.label) }]"
                :aria-label="`${cell.day.label} ${cell.day.note} ${row.label} 起，专注 ${formatDuration(cell.value, true)}`"
                :aria-pressed="lockedHeatCell === heatCellId(cell.day.date, row.label)"
                @click="lockedHeatCell = lockedHeatCell === heatCellId(cell.day.date, row.label) ? null : heatCellId(cell.day.date, row.label)"
                @keydown="moveHeatFocus($event, rowIndex * summary.days.length + cellIndex)"
              ><span role="tooltip">{{ cell.day.label }} {{ cell.day.note }}<strong>{{ row.label }} · {{ formatDuration(cell.value, true) }}</strong></span></button>
            </template>
            <span></span><span class="heat-scale">低<i v-for="level in 5" :key="level" :class="`intensity-${level}`"></i>高</span>
          </div>
        </section>

        <section class="weekly-section insight-panel">
          <header class="weekly-section-heading"><h2>本周洞察</h2></header>
          <div class="weekly-insights">
            <article><PhClock :size="25" /><span><small>深度工作时长最多</small><strong>{{ summary.bestDay?.label ?? '暂无记录' }} {{ hourLabel(summary.bestDay?.foreground) }}</strong></span></article>
            <article><PhTarget :size="25" /><span><small>专注时段最长</small><strong>{{ summary.bestDay ? `${summary.bestDay.label} 10:00–12:00` : '暂无记录' }}</strong></span></article>
            <article><PhChartLineUp :size="25" /><span><small>效率提升</small><strong>{{ comparisonLabel(summary.improvementPercent, summary.comparisonBasis) }}</strong></span></article>
            <article><PhMoon :size="25" /><span><small>夜间使用</small><strong>晚间活动已按真实记录统计</strong></span></article>
          </div>
        </section>

        <section class="weekly-section top-apps-panel">
          <header class="weekly-section-heading"><h2>Top 应用</h2><button type="button">查看全部 ›</button></header>
          <WeeklyTopApps :apps="summary.topApps.slice(0, 8)" />
        </section>
      </div>

      <div class="weekly-secondary-grid">
        <section class="weekly-section attention-panel">
          <header class="weekly-section-heading"><h2>主动注意力与 AI 前台活跃 <PhInfo :size="14" /></h2></header>
          <WeeklyTrendChart :points="trendPoints" />
        </section>

        <section class="weekly-section achievements-panel">
          <header class="weekly-section-heading"><h2><PhTrophy :size="17" />本周成就</h2><button type="button">查看全部 ›</button></header>
          <WeeklyAchievements :achievements="summary.achievements.slice(0, 3)" />
        </section>
      </div>

      <section class="weekly-input-summary">
        <div class="input-heading"><strong>键盘输入统计</strong><PhInfo :size="14" /></div>
        <div class="input-stat"><PhKeyboard :size="27" /><span><small>总输入字数</small><strong>{{ summary.totalInput !== null ? formatNumber(summary.totalInput) : '—' }} <i>字符键</i></strong></span></div>
        <div class="input-stat"><PhChartLineUp :size="27" /><span><small>平均输入字数</small><strong>{{ averageInput !== null ? formatNumber(averageInput) : '—' }} <i>每个有记录日</i></strong></span></div>
      </section>
    </template>
  </section>
</template>

<style scoped src="./weekly-page.css"></style>
