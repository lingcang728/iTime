<script setup lang="ts">
import { computed } from 'vue'
import { PhChartLineUp, PhKeyboard } from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import InputHistoryPanel from '../components/input/InputHistoryPanel.vue'
import { useAppStore } from '../stores/appStore'
import { hasInputData } from '../stores/dataAvailability'
import { formatNumber } from '../utils/format'

const store = useAppStore()
const snapshot = computed(() => store.input.value)
const inputDataAvailable = computed(() => hasInputData(store.state.inputDataStatus))
const averageCharacters = computed(() => {
  const activeMinutes = snapshot.value.history.filter((point) => point.keyStrokes > 0).length
  return activeMinutes ? snapshot.value.cumulative.keyStrokes / activeMinutes : 0
})
const pageSubtitle = computed(() => ({
  ready: '直接统计 Windows 字符键敲击；不保存键值或输入内容',
  preview: '展示浏览器预览数据；桌面版直接统计字符键敲击',
  loading: '正在连接 iTime 键盘计数器',
  unavailable: '键盘计数器暂时不可用',
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
    <PageHeader title="键盘输入统计" :subtitle="pageSubtitle" />

    <div class="input-stat-strip">
      <MetricCard label="总输入字数" :value="metricValue(formatNumber(snapshot.cumulative.keyStrokes))" :detail="metricDetail('所选日期字符键敲击总量')" :icon="PhKeyboard" tone="accent" />
      <MetricCard label="平均输入字数" :value="metricValue(formatNumber(averageCharacters))" :detail="metricDetail('每个有输入记录的分钟')" :icon="PhChartLineUp" tone="neutral" />
    </div>

    <div v-if="!inputDataAvailable" class="section-state input-source-state" :data-state="store.state.inputDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.inputDataMessage }}</span>
    </div>
    <template v-else>
      <div class="input-history-wrap">
        <InputHistoryPanel :history="snapshot.history" :granularity="snapshot.capabilities.historyGranularity" />
      </div>
    </template>
  </section>
</template>

<style scoped>
.input-stat-strip {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin-top: 2px;
  border-block: 1px solid var(--border-soft);
}

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
  .input-stat-strip { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 680px) {
  .input-stat-strip { grid-template-columns: 1fr; }

  .input-stat-strip :deep(.metric-card + .metric-card) {
    border-top: 1px solid var(--border-soft);
    border-left: 0;
  }
}
</style>
