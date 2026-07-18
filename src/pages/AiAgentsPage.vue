<script setup lang="ts">
import { computed } from 'vue'
import {
  PhArrowUpRight,
  PhCaretDown,
  PhChartLineUp,
  PhClock,
  PhInfo,
  PhLightning,
  PhPulse,
  PhSparkle,
  PhStack,
  PhUser,
} from '@phosphor-icons/vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import type { AiInteractionInterval, AiToolStatus, AiToolSummary, AiWorkInterval, StatValue } from '../domain/events'
import { bestActivityWindow, peakConcurrencyWindow } from '../domain/intervals'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatClock, formatDuration, formatRatio } from '../utils/format'

interface DurationPart {
  amount: string
  unit?: string
}

const store = useAppStore()
const statusLabels: Record<AiToolStatus, string> = { running: '运行中', completed: '已完成', waiting: '仅前台观察' }
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const providerDataAvailable = computed(() => hasActivityData(store.state.providerDataStatus))
const timelineDataAvailable = computed(() => activityDataAvailable.value || providerDataAvailable.value)
const aiWork = computed(() => store.day.value.events
  .filter((event): event is AiWorkInterval => event.type === 'aiWork')
  .sort((first, second) => first.start - second.start))
const aiInteractions = computed(() => store.day.value.events
  .filter((event): event is AiInteractionInterval => event.type === 'aiInteraction')
  .sort((first, second) => first.start - second.start))
const activityRows = computed(() => {
  if (aiWork.value.length) {
    return aiWork.value.slice(0, 8).map((event) => ({
      ...event,
      title: `${event.toolName} Provider 执行`,
      detail: event.basis,
      icon: PhPulse,
    }))
  }
  return aiInteractions.value.slice(0, 8).map((event) => ({
    ...event,
    title: `${event.toolName} 前台活跃`,
    detail: event.basis,
    icon: PhUser,
  }))
})
const topTool = computed(() => [...store.day.value.aiTools]
  .sort((first, second) => (second.effectiveDuration || second.foregroundDuration) - (first.effectiveDuration || first.foregroundDuration))[0] ?? null)
const efficientWindow = computed(() => bestActivityWindow(
  aiWork.value.length ? aiWork.value : aiInteractions.value,
  store.day.value.range,
))
const concurrencyWindow = computed(() => peakConcurrencyWindow(aiWork.value))
const efficiencyRange = computed(() => efficientWindow.value
  ? `${formatClock(efficientWindow.value.range.start)}–${formatClock(efficientWindow.value.range.end)}`
  : '今日未检测到 AI 活动')
const bestConcurrencyRange = computed(() => concurrencyWindow.value
  ? `${formatClock(concurrencyWindow.value.range.start)}–${formatClock(concurrencyWindow.value.range.end)} · ${concurrencyWindow.value.concurrency} 个`
  : '今日无并发执行')
const aiShare = computed(() => {
  const total = store.day.value.totalDuration.value
  const coverage = store.day.value.aiCoverage.value
  if (!total || coverage === null) return null
  return coverage / total
})
const previousDay = computed(() => store.week.value.at(-2) ?? null)

function durationParts(value: number | null, zeroWhenConnected = false): DurationPart[] {
  if (value === null && zeroWhenConnected && providerDataAvailable.value) return [{ amount: '0', unit: '分钟' }]
  if (value === null) return [{ amount: '—', unit: '等待数据' }]
  const hours = value / 3_600_000
  if (hours < 1) return [{ amount: String(Math.round(value / 60_000)), unit: '分钟' }]
  return [{ amount: hours.toFixed(1), unit: '小时' }]
}

function metricDelta(current: StatValue, previous: StatValue, kind: 'duration' | 'ratio' | 'count'): string {
  if (current.value === null) return current.basis
  if (previous.value === null) return '今日首次检测到'
  const difference = current.value - previous.value
  const sign = difference > 0 ? '+' : difference < 0 ? '−' : ''
  if (kind === 'duration') return difference === 0 ? '与昨日持平' : `较昨日 ${sign}${formatDuration(Math.abs(difference), true)}`
  if (kind === 'ratio') return difference === 0 ? '与昨日持平' : `较昨日 ${sign}${Math.abs(difference).toFixed(1)}×`
  return difference === 0 ? '与昨日持平' : `较昨日 ${sign}${Math.abs(Math.round(difference))} 个`
}

function providerMetricValue(value: number | null, format: (input: number) => string): string {
  if (value !== null) return format(value)
  return providerDataAvailable.value ? format(0) : '—'
}

function toolDuration(tool: AiToolSummary): number {
  return tool.taskCount ? tool.effectiveDuration : tool.foregroundDuration
}

function toolEvidence(tool: AiToolSummary): string {
  return tool.taskCount
    ? `${tool.taskCount} 个 Provider 执行 · ${statusLabels[tool.status]}`
    : `仅前台观察 · ${Math.round(tool.confidence * 100)}% 置信度`
}

const insight = computed(() => {
  const interaction = formatDuration(store.day.value.aiInteraction.value, true)
  if (!aiWork.value.length) {
    return {
      title: `记录到 ${interaction} 的 AI 前台活跃`,
      detail: providerDataAvailable.value
        ? 'Provider 会话已连接，但今天尚未检测到执行事件；此处仅呈现可验证的前台活动。'
        : '当前仅呈现可验证的前台活动；Provider 不可用时不会推算后台执行或并发。',
    }
  }
  const effective = formatDuration(store.day.value.aiEffective.value, true)
  const coverage = formatDuration(store.day.value.aiCoverage.value, true)
  return {
    title: `Provider 累计执行 ${effective}，覆盖 ${coverage}`,
    detail: concurrencyWindow.value
      ? `峰值并发 ${concurrencyWindow.value.concurrency} 个；执行证据按自然时间去重后计算。`
      : '执行证据已按自然时间去重。',
  }
})
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 代理" subtitle="智能代理在后台为你执行任务，提升产出与效率。" />

    <div class="ai-metrics">
      <MetricCard label="AI 前台活跃" :value-parts="durationParts(store.day.value.aiInteraction.value)" :detail="activityDataAvailable ? metricDelta(store.day.value.aiInteraction, previousDay?.aiInteraction ?? store.day.value.aiInteraction, 'duration') : store.state.activityDataMessage" :icon="PhUser" visual="bars" info="设备活跃且 AI 工具处于前台的可验证时长。" />
      <MetricCard label="Provider 执行覆盖" :value-parts="durationParts(store.day.value.aiCoverage.value, true)" :detail="providerDataAvailable ? metricDelta(store.day.value.aiCoverage, previousDay?.aiCoverage ?? store.day.value.aiCoverage, 'duration') : store.state.providerDataMessage" :icon="PhStack" visual="bars" info="至少一个 Provider 报告执行的自然时间覆盖。" />
      <MetricCard label="AI 杠杆率" :value="providerMetricValue(store.day.value.aiLeverage.value, (value) => formatRatio(value))" :detail="providerDataAvailable ? metricDelta(store.day.value.aiLeverage, previousDay?.aiLeverage ?? store.day.value.aiLeverage, 'ratio') : store.state.providerDataMessage" :icon="PhChartLineUp" visual="ring" info="Provider 累计执行时长与 AI 前台活跃时长之比；没有执行时显示 0。" />
      <MetricCard label="最高并发" :value="providerMetricValue(store.day.value.maxConcurrency.value, (value) => `${Math.round(value)} 个`)" :detail="providerDataAvailable ? metricDelta(store.day.value.maxConcurrency, previousDay?.maxConcurrency ?? store.day.value.maxConcurrency, 'count') : store.state.providerDataMessage" :icon="PhPulse" visual="bars" info="同一时刻有执行证据的工具峰值；没有执行时显示 0。" />
    </div>

    <div v-if="!activityDataAvailable || !providerDataAvailable" class="ai-source-stack">
      <div v-if="!activityDataAvailable" class="section-state ai-source-state" :data-state="store.state.activityDataStatus">
        <strong>AI 前台活动来源</strong><span>{{ store.state.activityDataMessage }}</span>
      </div>
      <div v-if="!providerDataAvailable" class="section-state ai-source-state" :data-state="store.state.providerDataStatus">
        <strong>Provider 执行来源</strong><span>{{ store.state.providerDataMessage }}</span>
      </div>
    </div>

    <div v-if="timelineDataAvailable" class="ai-workspace-grid">
      <section class="ai-panel ai-timeline-panel" aria-labelledby="ai-evidence-title">
        <div class="ai-section-heading">
          <h2 id="ai-evidence-title">AI 活动时间线</h2>
          <button type="button" class="text-button">全部活动<PhCaretDown :size="13" weight="bold" aria-hidden="true" /></button>
        </div>
        <div v-if="activityRows.length" class="ai-activity-list" aria-label="AI 活动证据">
          <article v-for="row in activityRows" :key="row.id" class="ai-activity-row">
            <time>{{ formatClock(row.start) }}</time>
            <span class="ai-activity-marker" aria-hidden="true"></span>
            <span class="ai-task-icon"><component :is="row.icon" :size="17" weight="regular" /></span>
            <div><strong>{{ row.title }}</strong><small>{{ row.toolName }} · {{ row.detail }}</small></div>
            <span class="ai-activity-meta"><b>{{ formatDuration(row.end - row.start, true) }}</b><small>置信度 {{ Math.round(row.confidence * 100) }}%</small></span>
          </article>
          <p class="ai-activity-note"><PhInfo :size="15" />{{ aiWork.length ? '执行区间来自 Codex/Claude Code 本机会话时间事件，不读取会话内容。' : '今天没有 Provider 执行事件，当前展示可验证的 AI 前台活跃。' }}</p>
        </div>
        <div v-else class="ai-tool-list__empty">今天尚未检测到 AI 工具活动。</div>
      </section>

      <section class="ai-panel ai-tools-panel" aria-labelledby="ai-tools-title">
        <div class="ai-section-heading">
          <h2 id="ai-tools-title">使用的工具</h2>
          <button type="button" class="text-button">按使用时长<PhCaretDown :size="13" weight="bold" aria-hidden="true" /></button>
        </div>
        <div v-if="store.day.value.aiTools.length" class="ai-tool-list" aria-label="AI 工具采样明细">
          <article v-for="tool in store.day.value.aiTools" :key="tool.toolId" class="ai-tool-item">
            <button type="button" :aria-label="`查看 ${tool.toolName} 采样详情`" @click="store.openTool(tool.toolId)">
              <ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="30" />
              <span class="ai-tool-copy"><strong>{{ tool.toolName }}</strong><small>{{ tool.taskCount ? `${tool.taskCount} 个本机会话执行区间` : '仅观察到前台活动' }}</small></span>
              <span class="ai-tool-meta"><b>{{ formatDuration(toolDuration(tool), true) }}</b><small>{{ toolEvidence(tool) }}</small></span>
              <PhArrowUpRight :size="15" aria-hidden="true" />
            </button>
          </article>
          <button class="ai-tools-more" type="button">查看所有工具使用明细<PhArrowUpRight :size="15" /></button>
        </div>
        <div v-else class="ai-tool-list__empty">当天尚未采集到 AI 工具活动。</div>
      </section>
    </div>

    <article v-if="activityDataAvailable" class="ai-insight">
      <span class="ai-insight__icon"><PhSparkle :size="21" weight="fill" /></span>
      <div class="ai-insight__copy"><span>今日洞察</span><h2>{{ insight.title }}</h2><p>{{ insight.detail }}<template v-if="topTool"> {{ topTool.toolName }} 是今天贡献最高的工具。</template></p></div>
      <dl>
        <div><dt><PhClock :size="18" />高效时段</dt><dd>{{ efficiencyRange }}</dd></div>
        <div><dt><PhChartLineUp :size="18" />AI 执行占比</dt><dd>{{ formatRatio(aiShare) }}</dd></div>
        <div><dt><PhLightning :size="18" />最佳并发时段</dt><dd>{{ bestConcurrencyRange }}</dd></div>
      </dl>
    </article>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
