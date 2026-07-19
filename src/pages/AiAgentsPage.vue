<script setup lang="ts">
import { computed } from 'vue'
import { PhChartLineUp, PhClock, PhPulse, PhRobot, PhStack, PhUser } from '@phosphor-icons/vue'
import AiActivityTimeline from '../components/AiActivityTimeline.vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import TimelineLane from '../components/TimelineLane.vue'
import type { AiInteractionInterval, AiWorkInterval, ForegroundAppInterval, StatValue, TimelineKind } from '../domain/events'
import { mergeRanges } from '../domain/intervals'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatRatio } from '../utils/format'

interface DurationPart { amount: string; unit?: string }

const store = useAppStore()
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const providerDataAvailable = computed(() => hasActivityData(store.state.providerDataStatus))
const aiWork = computed(() => store.day.value.events.filter((event): event is AiWorkInterval => event.type === 'aiWork').sort((a, b) => a.start - b.start))
const aiInteractions = computed(() => store.day.value.events.filter((event): event is AiInteractionInterval => event.type === 'aiInteraction').sort((a, b) => a.start - b.start))
const foreground = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const timelineRows = computed(() => [
  { label: '总览', meta: '两类证据', kind: 'overlap' as TimelineKind, segments: mergeRanges([...aiWork.value, ...aiInteractions.value]) },
  { label: 'Provider', meta: `${aiWork.value.length} 个区间`, kind: 'agent' as TimelineKind, segments: mergeRanges(aiWork.value) },
  { label: 'AI 前台', meta: `${aiInteractions.value.length} 个区间`, kind: 'interaction' as TimelineKind, segments: mergeRanges(aiInteractions.value) },
  { label: '并发', meta: `峰值 ${Math.round(store.day.value.maxConcurrency.value ?? 0)} 个`, kind: 'overlap' as TimelineKind, segments: aiWork.value },
])
const rankedTools = computed(() => [...store.day.value.aiTools].sort((a, b) => (b.effectiveDuration || b.foregroundDuration) - (a.effectiveDuration || a.foregroundDuration)))
const providerTotal = computed(() => store.day.value.aiEffective.value ?? 0)
const axisTicks = ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00']

function durationParts(value: number | null, connected = false): DurationPart[] {
  if (value === null && !connected) return [{ amount: '—', unit: '等待数据' }]
  const minutes = Math.max(0, Math.round((value ?? 0) / 60_000))
  return minutes >= 60
    ? [{ amount: String(Math.floor(minutes / 60)), unit: '小时' }, { amount: String(minutes % 60), unit: '分' }]
    : [{ amount: String(minutes), unit: '分钟' }]
}

function metricDelta(current: StatValue, previous: StatValue, kind: 'duration' | 'ratio' | 'count'): string {
  if (current.value === null) return current.basis
  if (previous.value === null) return '昨日暂无同类证据'
  const difference = current.value - previous.value
  if (!difference) return '与昨日持平'
  const sign = difference > 0 ? '+' : '−'
  if (kind === 'duration') return `较昨日 ${sign}${formatDuration(Math.abs(difference), true)}`
  if (kind === 'ratio') return `较昨日 ${sign}${Math.abs(difference).toFixed(1)}×`
  return `较昨日 ${sign}${Math.abs(Math.round(difference))} 个`
}

function toolDuration(tool: (typeof rankedTools.value)[number]): number {
  return tool.effectiveDuration || tool.foregroundDuration
}

function toolScope(tool: (typeof rankedTools.value)[number]): string {
  if (tool.taskCount) return `Provider · ${tool.taskCount} 个任务证据`
  return '仅前台观察'
}
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 代理" subtitle="把 Provider 执行证据与 AI 前台观察分开，看清真实可验证的协作轨迹。" />

    <div class="ai-metrics">
      <MetricCard label="AI 前台活跃" :value-parts="durationParts(store.day.value.aiInteraction.value, activityDataAvailable)" :detail="activityDataAvailable ? metricDelta(store.day.value.aiInteraction, store.previousDay.value.aiInteraction, 'duration') : store.state.activityDataMessage" :icon="PhUser" visual="bars" info="设备活跃时，被识别为 AI 工具的前台交互区间。" />
      <MetricCard label="Provider 累计执行" :value-parts="durationParts(store.day.value.aiEffective.value, providerDataAvailable)" :detail="providerDataAvailable ? metricDelta(store.day.value.aiEffective, store.previousDay.value.aiEffective, 'duration') : store.state.providerDataMessage" :icon="PhRobot" tone="accent" visual="bars" info="各 Provider 执行区间直接相加，并行时允许重叠。" />
      <MetricCard label="Provider 覆盖" :value-parts="durationParts(store.day.value.aiCoverage.value, providerDataAvailable)" :detail="providerDataAvailable ? metricDelta(store.day.value.aiCoverage, store.previousDay.value.aiCoverage, 'duration') : store.state.providerDataMessage" :icon="PhStack" visual="bars" info="至少一个 Provider 执行的自然时间并集，不重复计算并行区间。" />
      <MetricCard label="最高并发" :value="providerDataAvailable ? `${Math.round(store.day.value.maxConcurrency.value ?? 0)} 个` : '—'" :detail="providerDataAvailable ? metricDelta(store.day.value.maxConcurrency, store.previousDay.value.maxConcurrency, 'count') : store.state.providerDataMessage" :icon="PhPulse" visual="bars" info="同一时刻存在执行证据的 Provider 区间峰值。" />
      <MetricCard label="AI 杠杆率" :value="store.day.value.aiLeverage.value === null ? '—' : formatRatio(store.day.value.aiLeverage.value)" :detail="providerDataAvailable && activityDataAvailable ? metricDelta(store.day.value.aiLeverage, store.previousDay.value.aiLeverage, 'ratio') : '需要 Provider 与 AI 前台两类证据'" :icon="PhChartLineUp" visual="ring" info="Provider 累计执行 ÷ AI 前台活跃；缺少任一类证据时不推算。" />
    </div>

    <div v-if="!activityDataAvailable || !providerDataAvailable" class="ai-source-stack">
      <div v-if="!activityDataAvailable" class="section-state"><strong>AI 前台来源不可用</strong><span>{{ store.state.activityDataMessage }}</span></div>
      <div v-if="!providerDataAvailable" class="section-state"><strong>Provider 来源不可用</strong><span>{{ store.state.providerDataMessage }}</span></div>
    </div>

    <div class="ai-overview-grid">
      <article class="ai-evidence-card card">
        <div class="section-heading"><div><h2>AI 证据时间线</h2><p>同一日内的总览、Provider、前台与并发轨道</p></div><span class="evidence-badge">不读取会话内容</span></div>
        <div class="ai-axis" aria-hidden="true"><span></span><div><i v-for="tick in axisTicks" :key="tick">{{ tick }}</i></div></div>
        <div class="ai-timeline-rows">
          <div v-for="row in timelineRows" :key="row.label" class="ai-timeline-row">
            <span><strong>{{ row.label }}</strong><small>{{ row.meta }}</small></span>
            <TimelineLane :range="store.day.value.range" :segments="row.segments.map((segment) => ({ ...segment, kind: row.kind }))" />
          </div>
        </div>
        <div class="ai-legend"><span class="provider">Provider 执行</span><span class="foreground">AI 前台观察</span><span class="overlap">并行重叠</span></div>
      </article>

      <article class="ai-ranking-card card">
        <div class="section-heading"><div><h2>工具时长排行</h2><p>Provider 优先；无执行事件时仅列前台观察</p></div></div>
        <div v-if="rankedTools.length" class="ai-ranking-list">
          <button v-for="(tool, index) in rankedTools.slice(0, 7)" :key="tool.toolId" type="button" @click="store.openTool(tool.toolId)">
            <span class="ai-rank">{{ index + 1 }}</span><ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="24" />
            <span><strong>{{ tool.toolName }}</strong><small>{{ toolScope(tool) }}</small></span>
            <i><b :style="{ width: `${providerTotal ? tool.effectiveDuration / providerTotal * 100 : 100}%` }"></b></i>
            <time>{{ formatDuration(toolDuration(tool), true) }}</time>
          </button>
        </div>
        <div v-else class="section-state"><strong>今天没有 AI 活动证据</strong><span>不会用前台应用名称推算 Provider 后台执行。</span></div>
      </article>
    </div>

    <article class="ai-collaboration card">
      <div class="section-heading"><div><h2>真实执行区间协作图</h2><p>每条横线对应本机可验证区间；虚线等待不计入有效执行</p></div></div>
      <AiActivityTimeline :range="store.day.value.range" :foreground="foreground" :tools="rankedTools" />
    </article>

    <footer class="ai-boundary"><PhClock :size="16" /><span>Provider 证据来自 Codex / Claude Code 本机会话时间事件；前台观察来自 Windows 活动采集。二者口径不同，缺失时会分别降级显示。</span></footer>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
