<script setup lang="ts">
import { computed } from 'vue'
import PageHeader from '../components/PageHeader.vue'
import BarChart from '../components/BarChart.vue'
import FocusHeatmap from '../components/FocusHeatmap.vue'
import WeeklyAchievements from '../components/weekly/WeeklyAchievements.vue'
import WeeklyTopApps from '../components/weekly/WeeklyTopApps.vue'
import WeeklyTrendChart from '../components/weekly/WeeklyTrendChart.vue'
import { buildWeeklySummary } from '../components/weekly/weeklyModel'
import { uiIcons } from '../data/uiIcons'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatNumber } from '../utils/format'
import { buildFocusHeatmap } from '../data/focusHeatmap'

const store = useAppStore()
const hour = 3_600_000
const summary = computed(() => buildWeeklySummary(store.week.value))
const activityPoints = computed(() => summary.value.days.map((day) => ({
  label: day.label,
  value: day.computer === null ? null : day.computer / hour,
  note: day.note,
})))
const comparePoints = computed(() => summary.value.days.map((day) => ({
  label: day.label,
  value: day.foreground === null ? null : day.foreground / hour,
  secondary: day.ai === null ? null : day.ai / hour,
  note: day.note,
})))
const trendPoints = computed(() => summary.value.days.map((day) => ({
  label: day.label,
  note: day.note,
  value: day.foreground === null ? null : day.foreground / hour,
})))
const focusDays = computed(() => buildFocusHeatmap(store.state.selectedDate, summary.value.focusSamples))

function hourLabel(value: number | null | undefined): string {
  return value === null || value === undefined ? '暂无数据' : `${(value / hour).toFixed(1)} 小时`
}

function comparisonLabel(value: number | null, basis: 'previousWeek' | 'peerDays' | null): string {
  if (value === null) return '暂无上周基线'
  const reference = basis === 'peerDays' ? '本周其余有记录日平均' : '上周'
  return `较${reference}${value >= 0 ? '提升' : '下降'} ${Math.abs(value)}%`
}
</script>

<template>
  <section class="page weekly-page">
    <PageHeader title="本周回顾" subtitle="用可验证的本机记录回看专注、应用与 AI 工具使用" :range-label="summary.rangeLabel" />

    <div class="weekly-chart-stack">
      <article class="card weekly-card weekly-card--activity">
        <header class="weekly-card__heading"><div><span>电脑活动</span><h2>每日电脑活动</h2><p>设备活跃与空闲区间的去重时长</p></div><em class="legend legend--blue"><i></i>电脑活动</em></header>
        <BarChart :points="activityPoints" unit="小时" tone="blue" primary-label="电脑活动" />
      </article>
      <article class="card weekly-card">
        <header class="weekly-card__heading"><div><span>注意力构成</span><h2>主动注意力与 AI 前台活跃</h2><p>两组时长可重叠，用于核对工具处于前台的时间</p></div><div class="legend-group"><em class="legend legend--green"><i></i>主动注意力</em><em class="legend legend--violet"><i></i>AI 前台活跃</em></div></header>
        <BarChart :points="comparePoints" unit="小时" tone="green" primary-label="主动注意力" secondary-label="AI 前台活跃" compact />
      </article>
    </div>

    <div class="weekly-analysis-grid">
      <article class="card weekly-card focus-panel">
        <header class="weekly-card__heading"><div><span>专注分布</span><h2>专注热力图</h2><p>仅对已采集的前台活动着色，空白日期不会补造</p></div></header>
        <FocusHeatmap :days="focusDays" />
      </article>
      <article class="card weekly-card insight-panel">
        <header class="weekly-card__heading">
          <div>
            <span>本周洞察</span>
            <h2 class="weekly-insight-title">
              <img class="weekly-inline-icon" :src="uiIcons.weeklyBestDay" alt="" draggable="false" />
              最专注的一天是 <strong>{{ summary.bestDay?.label ?? '暂无记录' }}</strong>
            </h2>
          </div>
        </header>
        <div class="attention-summary">
          <span><small>当日主动注意力</small><strong>{{ hourLabel(summary.bestDay?.foreground) }}</strong></span>
          <span><small>本周主动注意力</small><strong>{{ hourLabel(summary.totalAttention) }}</strong></span>
          <em :class="{ positive: (summary.improvementPercent ?? 0) > 0, negative: (summary.improvementPercent ?? 0) < 0 }">{{ comparisonLabel(summary.improvementPercent, summary.comparisonBasis) }}</em>
        </div>
        <WeeklyTrendChart :points="trendPoints" />
      </article>
    </div>

    <article class="card weekly-card top-apps-panel">
      <header class="weekly-card__heading"><div><span>应用排行</span><h2>Top 应用</h2><p>按本周前台活跃时长排序，最多显示 10 个真实应用</p></div><em>{{ summary.topApps.length }} 个有记录应用</em></header>
      <WeeklyTopApps :apps="summary.topApps" />
    </article>

    <article class="card weekly-card achievements-panel">
      <header class="weekly-card__heading">
        <div class="weekly-heading-with-icon">
          <img class="weekly-section-icon" :src="uiIcons.weeklyAchievements" alt="" draggable="false" />
          <div><span>里程碑</span><h2>本周成就</h2><p>按明确阈值自动解锁，未达成项目保持中性状态</p></div>
        </div>
      </header>
      <WeeklyAchievements :achievements="summary.achievements" />
    </article>

    <article class="weekly-input-card">
      <div class="weekly-input-card__icon weekly-input-card__icon--art"><img :src="uiIcons.inputKeystrokes" alt="" draggable="false" /></div>
      <div><span>输入节奏</span><h2 v-if="summary.totalInput !== null">本周共有 {{ formatNumber(summary.totalInput) }} 次键盘敲击</h2><h2 v-else>本周尚无同源输入记录</h2><p v-if="summary.peakInputDay">输入最多的是 {{ summary.peakInputDay.label }}，共 {{ formatNumber(summary.peakInputDay.input ?? 0) }} 次。</p><p v-else>接入活动轨道后的输入统计会在这里形成周趋势。</p></div>
      <strong v-if="summary.totalAttention !== null">专注累计 {{ formatDuration(summary.totalAttention, true) }}</strong>
    </article>
  </section>
</template>

<style scoped>
@import './weekly-page.css';

.weekly-insight-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.weekly-inline-icon {
  width: 22px;
  height: 22px;
  object-fit: contain;
  filter: drop-shadow(0 1px 2px rgba(34, 38, 45, 0.12));
}

.weekly-heading-with-icon {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.weekly-section-icon {
  width: 28px;
  height: 28px;
  flex: 0 0 28px;
  margin-top: 2px;
  object-fit: contain;
  filter: drop-shadow(0 2px 3px rgba(34, 38, 45, 0.1));
}

.weekly-input-card__icon--art {
  background: color-mix(in srgb, var(--bg-soft) 88%, transparent) !important;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--border-soft) 65%, transparent);
}

.weekly-input-card__icon--art img {
  width: 26px;
  height: 26px;
  object-fit: contain;
  filter: drop-shadow(0 2px 3px rgba(34, 38, 45, 0.1));
}
</style>
