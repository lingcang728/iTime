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
import { formatClock, formatDuration, formatNumber } from '../utils/format'

const store = useAppStore()
const hour = 3_600_000
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const sourceStateTitle = computed(() => {
  if (store.state.activityDataStatus === 'loading') return '正在读取所选周期的活动记录'
  if (store.state.activityDataStatus === 'empty') return '所选周期暂无活动记录'
  return '所选周期活动记录读取失败'
})
// 传入上一周期基线，否则 improvementPercent 永远为 null、周对比恒显示无基线。
const summary = computed(() => buildWeeklySummary(store.week.value, store.previousWeek.value))
const trendPoints = computed(() => summary.value.days.map((day) => ({
  label: day.label,
  note: day.note,
  attention: day.foreground === null ? null : day.foreground / hour,
  ai: day.ai === null ? null : day.ai / hour,
})))
const averageBuckets = computed(() => Array.from({ length: 10 }, (_, index) => {
  const values = summary.value.days.flatMap((day) => day.computerBuckets ? [day.computerBuckets[index] ?? 0] : [])
  return values.length ? values.reduce((total, value) => total + value, 0) / values.length : 0
}))
const bucketMaximum = computed(() => Math.max(
  1,
  ...summary.value.days.flatMap((day) => day.computerBuckets ?? []),
  ...averageBuckets.value,
))
const averageComputer = computed(() => {
  const values = summary.value.days.flatMap((day) => day.computer === null ? [] : [day.computer])
  return values.length ? values.reduce((total, value) => total + value, 0) / values.length : null
})
const activeInputDays = computed(() => summary.value.days.filter((day) => (day.input ?? 0) > 0).length)
const averageInput = computed(() => summary.value.totalInput === null || !activeInputDays.value
  ? null
  : summary.value.totalInput / activeInputDays.value)
const heatHours = [9, 11, 13, 15, 17, 19, 21]
// P3-48: 49 个热力格只占一个 Tab 位，方向键在格间移动（roving tabindex）。
const rovingHeatIndex = ref(0)
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

function comparisonLabel(value: number | null, basis: 'previousWeek' | null): string {
  if (value === null || basis === null) return '暂无上一周期基线'
  return `与上一周期相比，前台专注时长 ${value >= 0 ? '↑' : '↓'} ${Math.abs(value)}%`
}

function sparkHeight(value: number): number {
  return value ? Math.max(8, value / bucketMaximum.value * 100) : 3
}

function heatCellId(date: string, label: string): string {
  return `${date}-${label}`
}

function moveHeatFocus(event: KeyboardEvent, index: number): void {
  const offset = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 }[event.key]
    ?? (event.key === 'Home' ? -index : event.key === 'End' ? 48 - index : undefined)
  if (offset === undefined || !(event.currentTarget instanceof HTMLButtonElement)) return
  event.preventDefault()
  const target = Math.max(0, Math.min(48, index + offset))
  rovingHeatIndex.value = target
  const cells = event.currentTarget.closest('.weekly-heatmap')?.querySelectorAll<HTMLButtonElement>('.heat-cell')
  cells?.[target]?.focus()
}
</script>

<template>
  <section class="page weekly-page">
    <PageHeader title="周报" subtitle="以所选日期为结尾的 7 天汇总" :range-label="summary.rangeLabel" />

    <div v-if="!activityDataAvailable" class="section-state weekly-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ sourceStateTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <template v-else>
      <section class="weekly-section weekly-daily-card" aria-labelledby="daily-title">
        <h2 id="daily-title">每日活动</h2>
        <div class="weekly-days">
          <article v-for="day in summary.days" :key="day.date" :class="{ best: day.date === summary.bestDay?.date }">
            <span><PhStar v-if="day.date === summary.bestDay?.date" :size="12" weight="fill" aria-hidden="true" />{{ day.label }}<small>{{ day.note }}</small></span>
            <strong>{{ day.computer === null ? '—' : (day.computer / hour).toFixed(1) }}<small v-if="day.computer !== null">小时</small></strong>
            <div v-if="day.computerBuckets" class="daily-spark" aria-hidden="true"><i v-for="(value, index) in day.computerBuckets" :key="index" :style="{ height: `${sparkHeight(value)}%` }"></i></div>
          </article>
          <article class="average-day">
            <span>平均</span>
            <strong>{{ averageComputer === null ? '—' : (averageComputer / hour).toFixed(1) }}<small v-if="averageComputer !== null">小时</small></strong>
            <div v-if="summary.days.some((day) => day.computerBuckets)" class="daily-spark" aria-hidden="true"><i v-for="(value, index) in averageBuckets" :key="index" :style="{ height: `${sparkHeight(value)}%` }"></i></div>
          </article>
        </div>
      </section>

      <div class="weekly-analysis-grid">
        <section class="weekly-section focus-panel">
          <header class="weekly-section-heading"><h2>专注热力图 <PhInfo :size="14" aria-hidden="true" /></h2><span>统计 09:00–23:00</span></header>
          <!-- 扁平按钮组 + 方向键漫游；不用 role="grid"，避免缺失 row/gridcell 的虚位语义。 -->
          <div class="weekly-heatmap" role="group" :aria-label="`${summary.rangeLabel} 分时专注热力图，仅覆盖白天 09:00 到 23:00`">
            <span></span><span v-for="day in summary.days" :key="day.date" class="heat-day">{{ day.label }}<small>{{ day.note }}</small></span>
            <template v-for="(row, rowIndex) in hourlyHeat" :key="row.label">
              <span class="heat-hour">{{ row.label }}</span>
              <button
                v-for="(cell, cellIndex) in row.cells"
                :key="heatCellId(cell.day.date, row.label)"
                type="button"
                class="heat-cell"
                :class="`intensity-${cell.intensity}`"
                :tabindex="rowIndex * summary.days.length + cellIndex === rovingHeatIndex ? 0 : -1"
                @focus="rovingHeatIndex = rowIndex * summary.days.length + cellIndex"
                :aria-label="`${cell.day.label} ${cell.day.note} ${row.label} 起，专注 ${formatDuration(cell.value, true)}`"
                @keydown="moveHeatFocus($event, rowIndex * summary.days.length + cellIndex)"
              ><span role="tooltip">{{ cell.day.label }} {{ cell.day.note }}<strong>{{ row.label }} · {{ formatDuration(cell.value, true) }}</strong></span></button>
            </template>
            <span></span><span class="heat-scale">低<i v-for="level in 5" :key="level" :class="`intensity-${level}`"></i>高</span>
          </div>
        </section>

        <section class="weekly-section insight-panel">
          <header class="weekly-section-heading"><h2>{{ summary.rangeLabel }}</h2></header>
          <div class="weekly-insights">
            <article><PhClock :size="25" aria-hidden="true" /><span><small>专注最多</small><strong>{{ summary.bestDay?.label ?? '—' }} {{ hourLabel(summary.bestDay?.foreground) }}</strong></span></article>
            <article><PhTarget :size="25" aria-hidden="true" /><span><small>最长连续</small><strong>{{ summary.longestForeground ? `${summary.longestForeground.label} ${formatClock(summary.longestForeground.start)}–${formatClock(summary.longestForeground.end)}` : '—' }}</strong></span></article>
            <article><PhChartLineUp :size="25" aria-hidden="true" /><span><small>周对比</small><strong>{{ comparisonLabel(summary.improvementPercent, summary.comparisonBasis) }}</strong></span></article>
            <article><PhMoon :size="25" aria-hidden="true" /><span><small>夜间（00–06 / 22–24）</small><strong>{{ summary.nightForeground === null ? '—' : formatDuration(summary.nightForeground, true) }}</strong></span></article>
          </div>
        </section>

        <section class="weekly-section top-apps-panel">
          <header class="weekly-section-heading"><h2>Top 应用</h2><span>Top 8</span></header>
          <WeeklyTopApps :apps="summary.topApps.slice(0, 8)" />
        </section>
      </div>

      <div class="weekly-secondary-grid">
        <section class="weekly-section attention-panel">
          <header class="weekly-section-heading"><h2>专注与 AI <PhInfo :size="14" aria-hidden="true" /></h2></header>
          <WeeklyTrendChart :points="trendPoints" />
        </section>

        <section class="weekly-section achievements-panel">
          <header class="weekly-section-heading"><h2><PhTrophy :size="17" aria-hidden="true" />成就</h2><span>Top 3</span></header>
          <WeeklyAchievements :achievements="summary.achievements.slice(0, 3)" />
        </section>
      </div>

      <section class="weekly-input-summary">
        <div class="input-heading"><strong>键盘</strong><PhInfo :size="14" aria-hidden="true" /></div>
        <div class="input-stat"><PhKeyboard :size="27" aria-hidden="true" /><span><small>字符键合计</small><strong>{{ summary.totalInput !== null ? formatNumber(summary.totalInput) : '—' }} <i>次</i></strong></span></div>
        <div class="input-stat"><PhChartLineUp :size="27" aria-hidden="true" /><span><small>有记录日均</small><strong>{{ averageInput !== null ? formatNumber(averageInput) : '—' }} <i>次 / 日</i></strong></span></div>
      </section>
    </template>
  </section>
</template>

<style scoped src="./weekly-page.css"></style>
