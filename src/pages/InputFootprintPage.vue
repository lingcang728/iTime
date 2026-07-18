<script setup lang="ts">
import { computed } from 'vue'
import {
  PhCursorClick,
  PhKeyboard,
  PhMouse,
  PhMouseLeftClick,
  PhMouseRightClick,
  PhMouseScroll,
} from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import InputHistoryPanel from '../components/input/InputHistoryPanel.vue'
import KeyboardInsights from '../components/input/KeyboardInsights.vue'
import { useAppStore } from '../stores/appStore'
import { hasInputData } from '../stores/dataAvailability'
import { formatDistance, formatNumber } from '../utils/format'

const store = useAppStore()
const snapshot = computed(() => store.input.value)
const hasSplitClicks = computed(() => snapshot.value.cumulative.leftClicks !== null && snapshot.value.cumulative.rightClicks !== null)
const inputDataAvailable = computed(() => hasInputData(store.state.inputDataStatus))
const pageSubtitle = computed(() => ({
  ready: '读取 iTime 本机聚合记录；不记录输入内容',
  preview: '展示浏览器预览数据；不记录输入内容',
  loading: '正在连接 iTime 本机输入记录',
  unavailable: '本机输入记录暂时不可用',
}[store.state.inputDataStatus]))
const unavailableTitle = computed(() => store.state.inputDataStatus === 'loading' ? '正在读取输入记录' : '输入记录暂不可用')

function metricValue(value: string): string {
  return inputDataAvailable.value ? value : '—'
}

function metricDetail(detail: string): string {
  return inputDataAvailable.value ? detail : store.state.inputDataMessage
}
</script>

<template>
  <section class="page input-page">
    <PageHeader title="输入足迹" :subtitle="pageSubtitle" />

    <div class="input-stat-strip" :class="{ 'input-stat-strip--four': !hasSplitClicks }">
      <MetricCard label="键盘敲击" :value="metricValue(formatNumber(snapshot.cumulative.keyStrokes))" :detail="metricDetail('所选日期总量')" :icon="PhKeyboard" tone="accent" />
      <template v-if="hasSplitClicks">
        <MetricCard label="鼠标左键点击" :value="metricValue(formatNumber(snapshot.cumulative.leftClicks))" :detail="metricDetail('所选日期总量')" :icon="PhMouseLeftClick" tone="neutral" />
        <MetricCard label="鼠标右键点击" :value="metricValue(formatNumber(snapshot.cumulative.rightClicks))" :detail="metricDetail('所选日期总量')" :icon="PhMouseRightClick" tone="neutral" />
      </template>
      <MetricCard v-else label="鼠标点击" :value="metricValue(formatNumber(snapshot.cumulative.combinedClicks))" :detail="metricDetail('历史记录仅提供合计')" :icon="PhCursorClick" tone="neutral" />
      <MetricCard label="鼠标移动" :value="metricValue(formatDistance(snapshot.cumulative.mouseDistance))" :detail="metricDetail('聚合距离')" :icon="PhMouse" tone="neutral" />
      <MetricCard label="滚动距离" :value="metricValue(formatNumber(snapshot.cumulative.scrollDistance))" :detail="metricDetail('数据源滚动单位')" :icon="PhMouseScroll" tone="neutral" />
    </div>

    <div v-if="!inputDataAvailable" class="section-state input-source-state" :data-state="store.state.inputDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.inputDataMessage }}</span>
    </div>
    <template v-else>
      <div class="input-history-wrap">
        <InputHistoryPanel :history="snapshot.history" :granularity="snapshot.capabilities.historyGranularity" />
      </div>

      <KeyboardInsights
        :snapshot="snapshot"
        :heatmap-enabled="store.state.heatmapEnabled"
        :shortcuts-enabled="store.state.shortcutsEnabled"
      />
    </template>
  </section>
</template>

<style scoped>
.input-stat-strip {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  margin-top: 2px;
  border-block: 1px solid var(--border-soft);
}

.input-stat-strip--four { grid-template-columns: repeat(4, minmax(0, 1fr)); }

.input-stat-strip :deep(.metric-card) {
  min-height: 112px;
  padding: 16px 18px;
  border: 0;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}

.input-stat-strip :deep(.metric-card + .metric-card) { border-left: 1px solid var(--border-soft); }
.input-stat-strip :deep(.metric-card__value) { white-space: nowrap; }
.input-history-wrap { margin-top: 20px; }
.input-source-state { margin-top: 20px; }

@media (max-width: 1040px) {
  .input-stat-strip :deep(.metric-value-number) { font-size: 23px; letter-spacing: -.6px; }
}

@media (max-width: 840px) {
  .input-stat-strip,
  .input-stat-strip--four { grid-template-columns: repeat(2, minmax(0, 1fr)); }

  .input-stat-strip :deep(.metric-card:nth-child(odd)) { border-left: 0; }
  .input-stat-strip :deep(.metric-card:nth-child(n + 3)) { border-top: 1px solid var(--border-soft); }
  .input-stat-strip:not(.input-stat-strip--four) :deep(.metric-card:last-child) { grid-column: 1 / -1; }
}

@media (max-width: 680px) {
  .input-stat-strip,
  .input-stat-strip--four { grid-template-columns: 1fr; }

  .input-stat-strip:not(.input-stat-strip--four) :deep(.metric-card:last-child) { grid-column: auto; }
  .input-stat-strip :deep(.metric-card + .metric-card) {
    border-top: 1px solid var(--border-soft);
    border-left: 0;
  }
}
</style>
