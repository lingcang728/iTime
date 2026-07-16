<script setup lang="ts">
import { computed } from 'vue'
import {
  PhArrowsDownUp,
  PhCursorClick,
  PhDatabase,
  PhKeyboard,
  PhMouse,
  PhMouseLeftClick,
  PhMouseRightClick,
  PhWarningCircle,
} from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import InputHistoryPanel from '../components/input/InputHistoryPanel.vue'
import KeyboardInsights from '../components/input/KeyboardInsights.vue'
import { useAppStore } from '../stores/appStore'
import { formatDistance, formatNumber } from '../utils/format'

const store = useAppStore()
const snapshot = computed(() => store.input.value)
const hasSplitClicks = computed(() => snapshot.value.cumulative.leftClicks !== null && snapshot.value.cumulative.rightClicks !== null)
const pageSubtitle = computed(() => store.state.inputDataStatus === 'ready'
  ? '读取本机 KeyStats 聚合记录；不记录输入内容'
  : '展示输入数据能力与聚合结果；不记录输入内容')
const sourceState = computed(() => store.state.inputDataStatus === 'ready' ? '本机只读' : '数据预览')
const sourceUpdated = computed(() => snapshot.value.sourceUpdatedAt
  ? new Intl.DateTimeFormat('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(snapshot.value.sourceUpdatedAt)
  : '暂无更新时间')
const granularityText = computed(() => ({
  minute: '分钟级历史',
  hour: '小时级历史',
  day: '日级历史',
  none: '无历史明细',
})[snapshot.value.capabilities.historyGranularity])
const timezoneNote = computed(() => snapshot.value.capabilities.timezoneSemantics === 'utc-date-bucket'
  ? 'KeyStats 按 UTC 日期归档，北京时间凌晨记录可能计入前一天。'
  : '记录按本地日期归档。')
</script>

<template>
  <section class="page input-page">
    <PageHeader title="输入足迹" :subtitle="pageSubtitle" />

    <div class="metrics-grid input-metrics">
      <MetricCard label="键盘敲击" :value="formatNumber(snapshot.cumulative.keyStrokes)" detail="所选日期总量" :icon="PhKeyboard" tone="green" />
      <template v-if="hasSplitClicks">
        <MetricCard label="鼠标左键点击" :value="formatNumber(snapshot.cumulative.leftClicks)" detail="所选日期总量" :icon="PhMouseLeftClick" tone="blue" />
        <MetricCard label="鼠标右键点击" :value="formatNumber(snapshot.cumulative.rightClicks)" detail="所选日期总量" :icon="PhMouseRightClick" tone="violet" />
      </template>
      <MetricCard v-else label="鼠标点击" :value="formatNumber(snapshot.cumulative.combinedClicks)" detail="历史记录仅提供合计" :icon="PhCursorClick" tone="blue" />
      <MetricCard label="鼠标移动" :value="formatDistance(snapshot.cumulative.mouseDistance)" detail="聚合距离" :icon="PhMouse" tone="cyan" />
      <MetricCard label="滚动距离" :value="formatNumber(snapshot.cumulative.scrollDistance)" detail="数据源滚动单位" :icon="PhArrowsDownUp" tone="orange" />
    </div>

    <div class="input-story-grid">
      <InputHistoryPanel :history="snapshot.history" :granularity="snapshot.capabilities.historyGranularity" />
      <article class="card source-card">
        <header><span><PhDatabase :size="18" weight="duotone" /></span><div><small>数据来源</small><h2>{{ snapshot.source }}</h2></div><b>{{ sourceState }}</b></header>
        <dl>
          <div><dt>最近更新</dt><dd>{{ sourceUpdated }}</dd></div>
          <div><dt>历史范围</dt><dd>{{ granularityText }}</dd></div>
          <div><dt>点击数据</dt><dd>{{ snapshot.capabilities.splitHistoricalClicks ? '左右键可拆分' : '历史仅合计' }}</dd></div>
        </dl>
        <p class="source-note"><PhWarningCircle :size="16" />{{ timezoneNote }}</p>
        <p v-if="!snapshot.capabilities.sensitiveSurfaceExclusion" class="source-note">数据源未提供按敏感界面排除统计的能力。</p>
      </article>
    </div>

    <KeyboardInsights
      :snapshot="snapshot"
      :heatmap-enabled="store.state.heatmapEnabled"
      :shortcuts-enabled="store.state.shortcutsEnabled"
    />
  </section>
</template>

<style scoped>
.input-metrics { grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); }
.input-story-grid { display: grid; grid-template-columns: minmax(0, 2.35fr) minmax(230px, .72fr); gap: 12px; margin-top: 12px; }
.source-card { min-width: 0; padding: 19px; }
.source-card header { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 10px; padding-bottom: 15px; border-bottom: 1px solid var(--border-soft); }
.source-card header > span { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 10px; color: var(--accent-blue); background: var(--accent-blue-soft); }
.source-card small { color: var(--text-muted); font-size: 8px; }
.source-card h2 { overflow: hidden; margin: 2px 0 0; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
.source-card header b { padding: 5px 7px; border-radius: 999px; color: var(--accent-green-strong); background: var(--accent-green-soft); font-size: 8px; }
.source-card dl { display: grid; gap: 9px; margin: 15px 0; }
.source-card dl div { display: flex; justify-content: space-between; gap: 12px; }
.source-card dt { color: var(--text-secondary); font-size: 9px; }
.source-card dd { margin: 0; font-size: 9px; font-weight: 650; text-align: right; }
.source-note { display: flex; align-items: flex-start; gap: 7px; margin: 9px 0 0; color: var(--text-secondary); font-size: 9px; line-height: 1.55; }
.source-note svg { flex: 0 0 auto; color: var(--accent-orange); }
@media (max-width: 980px) { .input-story-grid { grid-template-columns: 1fr; } }
</style>
