<script setup lang="ts">
import { computed } from 'vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import InputHistoryPanel from '../components/input/InputHistoryPanel.vue'
import KeyboardInsights from '../components/input/KeyboardInsights.vue'
import { uiIcons } from '../data/uiIcons'
import { useAppStore } from '../stores/appStore'
import { formatDistance, formatNumber } from '../utils/format'

const store = useAppStore()
const snapshot = computed(() => store.input.value)
const hasSplitClicks = computed(() => snapshot.value.cumulative.leftClicks !== null && snapshot.value.cumulative.rightClicks !== null)
const pageSubtitle = computed(() => store.state.inputDataStatus === 'ready'
  ? '读取 iTime 本机聚合记录；不记录输入内容'
  : '展示输入数据能力与聚合结果；不记录输入内容')
</script>

<template>
  <section class="page input-page">
    <PageHeader title="输入足迹" :subtitle="pageSubtitle" />

    <div class="metrics-grid input-metrics">
      <MetricCard label="键盘敲击" :value="formatNumber(snapshot.cumulative.keyStrokes)" detail="所选日期总量" :icon-src="uiIcons.inputKeystrokes" tone="green" />
      <template v-if="hasSplitClicks">
        <MetricCard label="鼠标左键点击" :value="formatNumber(snapshot.cumulative.leftClicks)" detail="所选日期总量" :icon-src="uiIcons.inputLeftClick" tone="blue" />
        <MetricCard label="鼠标右键点击" :value="formatNumber(snapshot.cumulative.rightClicks)" detail="所选日期总量" :icon-src="uiIcons.inputRightClick" tone="violet" />
      </template>
      <MetricCard v-else label="鼠标点击" :value="formatNumber(snapshot.cumulative.combinedClicks)" detail="历史记录仅提供合计" :icon-src="uiIcons.inputLeftClick" tone="blue" />
      <MetricCard label="鼠标移动" :value="formatDistance(snapshot.cumulative.mouseDistance)" detail="聚合距离" :icon-src="uiIcons.inputMouseMove" tone="cyan" />
      <MetricCard label="滚动距离" :value="formatNumber(snapshot.cumulative.scrollDistance)" detail="数据源滚动单位" :icon-src="uiIcons.inputScroll" tone="orange" />
    </div>

    <div class="input-history-wrap">
      <InputHistoryPanel :history="snapshot.history" :granularity="snapshot.capabilities.historyGranularity" />
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
.input-history-wrap { margin-top: 12px; }
</style>
