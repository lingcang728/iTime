<script setup lang="ts">
import { computed } from 'vue'
import { PhKeyboard, PhStar, PhTrophy } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import BarChart from '../components/BarChart.vue'
import FocusHeatmap from '../components/FocusHeatmap.vue'
import WeeklyAchievements from '../components/weekly/WeeklyAchievements.vue'
import WeeklyTopApps from '../components/weekly/WeeklyTopApps.vue'
import WeeklyTrendChart from '../components/weekly/WeeklyTrendChart.vue'
import { buildWeeklySummary } from '../components/weekly/weeklyModel'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatNumber } from '../utils/format'
import { buildFocusHeatmap } from '../data/focusHeatmap'

const store = useAppStore()
const hour = 3_600_000
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const pageSubtitle = computed(() => activityDataAvailable.value
  ? '用可验证的本机记录回看专注、应用与 AI 工具使用'
  : store.state.activityDataStatus === 'loading' ? '正在读取本周活动记录' : '本周活动记录暂时不可用')
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取本周活动记录' : '本周活动记录暂不可用')
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
    <PageHeader title="本周回顾" :subtitle="pageSubtitle" :range-label="summary.rangeLabel" />

    <div v-if="!activityDataAvailable" class="section-state weekly-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <template v-else>
    <section class="weekly-section weekly-section--activity">
      <header class="weekly-section-heading">
        <div>
          <span class="weekly-kicker">电脑活动</span>
          <h2>每日电脑活动</h2>
          <p>设备活跃与空闲区间的去重时长</p>
        </div>
        <em class="legend"><i></i>电脑活动</em>
      </header>
      <BarChart :points="activityPoints" unit="小时" tone="green" primary-label="电脑活动" />
    </section>

    <div class="weekly-analysis-grid">
      <section class="weekly-section focus-panel">
        <header class="weekly-section-heading">
          <div>
            <span class="weekly-kicker">专注分布</span>
            <h2>专注热力图</h2>
            <p>仅对已采集的前台活动着色，空白日期不会补造</p>
          </div>
        </header>
        <FocusHeatmap :days="focusDays" />
      </section>

      <section class="weekly-section insight-panel">
        <header class="weekly-section-heading">
          <div>
            <span class="weekly-kicker">本周洞察</span>
            <h2 class="weekly-insight-title">
              <PhStar :size="17" weight="regular" />
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
      </section>
    </div>

    <section class="weekly-section top-apps-panel">
      <header class="weekly-section-heading">
        <div>
          <span class="weekly-kicker">应用排行</span>
          <h2>Top 应用</h2>
          <p>按本周前台活跃时长排序，最多显示 10 个真实应用</p>
        </div>
        <em>{{ summary.topApps.length }} 个有记录应用</em>
      </header>
      <WeeklyTopApps :apps="summary.topApps" />
    </section>

    <div class="weekly-secondary-grid">
      <section class="weekly-section attention-panel">
        <header class="weekly-section-heading">
          <div>
            <span class="weekly-kicker">注意力构成</span>
            <h2>主动注意力与 AI 前台活跃</h2>
            <p>按日期对齐；两组时长可重叠，不代表重复统计</p>
          </div>
          <div class="legend-group">
            <em class="legend"><i></i>主动注意力</em>
            <em class="legend legend--neutral"><i></i>AI 前台活跃</em>
          </div>
        </header>
        <BarChart :points="comparePoints" unit="小时" tone="green" primary-label="主动注意力" secondary-label="AI 前台活跃" compact />
      </section>

      <section class="weekly-section achievements-panel">
        <header class="weekly-section-heading">
          <div class="weekly-heading-with-icon">
            <PhTrophy :size="18" weight="regular" />
            <div>
              <span class="weekly-kicker">里程碑</span>
              <h2>本周成就</h2>
              <p>按明确阈值自动解锁，未达成项目保持中性状态</p>
            </div>
          </div>
        </header>
        <WeeklyAchievements :achievements="summary.achievements" />
      </section>
    </div>

    <section class="weekly-input-summary">
      <PhKeyboard :size="20" weight="regular" />
      <div>
        <span>输入节奏</span>
        <h2 v-if="summary.totalInput !== null">本周共有 {{ formatNumber(summary.totalInput) }} 次键盘敲击</h2>
        <h2 v-else>本周尚无同源输入记录</h2>
        <p v-if="summary.peakInputDay">输入最多的是 {{ summary.peakInputDay.label }}，共 {{ formatNumber(summary.peakInputDay.input ?? 0) }} 次。</p>
        <p v-else>接入活动轨道后的输入统计会在这里形成周趋势。</p>
      </div>
      <strong v-if="summary.totalAttention !== null">专注累计 {{ formatDuration(summary.totalAttention, true) }}</strong>
    </section>
    </template>
  </section>
</template>

<style scoped>
@import './weekly-page.css';
</style>
