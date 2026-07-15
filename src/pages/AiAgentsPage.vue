<script setup lang="ts">
import { computed } from 'vue'
import { PhCaretRight, PhClock, PhRobot, PhStack, PhTrendUp } from '@phosphor-icons/vue'
import AgentMetricCard from '../components/AgentMetricCard.vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import PageHeader from '../components/PageHeader.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatRatio } from '../utils/format'
import { intersectRanges } from '../domain/intervals'
import type { ForegroundAppInterval, TimelineSegment } from '../domain/events'

const store = useAppStore()
const foregroundRanges = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const foregroundSegments = computed<TimelineSegment[]>(() => foregroundRanges.value.map((event) => ({ start: event.start, end: event.end, kind: 'attention' })))
const aiSegments = computed<TimelineSegment[]>(() => {
  const result: TimelineSegment[] = []
  for (const tool of store.day.value.aiTools) {
    result.push(...tool.workIntervals.map((event) => ({ start: event.start, end: event.end, kind: 'agent' as const, variant: 'solid' as const })))
    result.push(...tool.waitIntervals.map((event) => ({ ...event, kind: 'waiting' as const, variant: 'hatched' as const })))
    result.push(...tool.interactionIntervals.map((event) => ({ start: event.start, end: event.end, kind: 'interaction' as const, variant: 'outline' as const })))
    result.push(...intersectRanges(tool.workIntervals, foregroundRanges.value).map((event) => ({ ...event, kind: 'overlap' as const, variant: 'overlap' as const })))
  }
  return result
})
const timeTicks = [0, 4, 8, 12, 16, 20, 24]
const statusLabels = { running: '运行中', completed: '已完成', waiting: '等待中' }
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 代理时间" subtitle="让工具默默工作，你专注于重要的事" />
    <div class="metrics-grid metrics-grid--four ai-metrics-grid">
      <AgentMetricCard label="有效代理工时" :value="formatDuration(store.day.value.aiEffective.value, true)" detail="多代理工作区间累计" :icon="PhRobot" tone="violet" info="代理工时可重叠累计" />
      <AgentMetricCard label="工作覆盖时长" :value="formatDuration(store.day.value.aiCoverage.value, true)" detail="至少一个代理在工作" :icon="PhClock" tone="blue" />
      <AgentMetricCard label="AI 杠杆率" :value="formatRatio(store.day.value.aiLeverage.value)" detail="有效工时 ÷ 前台交互" :icon="PhTrendUp" tone="orange" />
      <AgentMetricCard label="最高并发数" :value="`${store.day.value.maxConcurrency.value ?? 0} 个`" detail="同一时刻有效代理" :icon="PhStack" tone="cyan" />
    </div>
    <article class="card ai-timeline-card">
      <div class="section-heading"><div><h2>AI 代理时间线</h2><p>与人的前台活动对比</p></div><div class="legend"><span class="green">人的前台活动</span><span class="violet">AI 代理工作</span></div></div>
      <div class="time-axis time-axis--endpoints"><span v-for="tick in timeTicks" :key="tick" :style="{ left: `${tick / 24 * 100}%` }">{{ String(tick).padStart(2, '0') }}:00</span></div>
      <TimelineLane label="人的活动" :range="store.day.value.range" :segments="foregroundSegments" />
      <TimelineLane label="工具的工作" :range="store.day.value.range" :segments="aiSegments" />
      <div class="timeline-note"><span class="state-interaction">前台交互</span><span class="state-agent">有效执行</span><span class="state-waiting">静默等待</span><span class="state-overlap">并行重叠</span></div>
    </article>
    <article class="card tools-card">
      <div class="section-heading"><div><h2>AI 代理与工具</h2><p>点击工具查看统计依据</p></div></div>
      <div class="tools-table" role="table">
        <div class="tools-table__head" role="row"><span>工具</span><span>状态</span><span>运行时长</span><span>前台交互</span><span>静默等待</span><span>并行重叠</span><span>展开</span></div>
        <div v-for="tool in store.day.value.aiTools" :key="tool.toolId" class="tools-table__row" role="row">
          <span class="tool-name-cell"><ApplicationIcon :icon-key="tool.iconKey" :size="22" /><strong>{{ tool.toolName }}</strong></span>
          <span :class="['status-text', `is-${tool.status}`]">{{ statusLabels[tool.status] }}</span>
          <span>{{ formatDuration(tool.effectiveDuration, true) }}</span>
          <span>{{ formatDuration(tool.foregroundDuration, true) }}</span>
          <span>{{ formatDuration(tool.silentWaitDuration, true) }}</span>
          <span>{{ formatDuration(tool.parallelOverlapDuration, true) }}</span>
          <button class="tool-expand" type="button" :aria-label="`展开 ${tool.toolName} 详细记录`" @click="store.openTool(tool.toolId)">展开<PhCaretRight :size="12" /></button>
        </div>
      </div>
    </article>
    <article class="ai-insight-card">
      <div><span>今日洞察</span><h2>AI 为你完成了 {{ formatDuration(store.day.value.aiEffective.value, true) }} 的有效工作</h2><p>覆盖现实时间 {{ formatDuration(store.day.value.aiCoverage.value, true) }}，并行增益 {{ formatRatio(store.day.value.parallelGain.value) }}。</p></div>
      <div class="gain-visual"><span style="height: 76%"></span><span style="height: 48%"></span></div>
    </article>
  </section>
</template>
