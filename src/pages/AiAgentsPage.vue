<script setup lang="ts">
import { computed } from 'vue'
import { PhClock, PhRobot, PhStack, PhTrendUp } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import MetricCard from '../components/MetricCard.vue'
import TimelineLane from '../components/TimelineLane.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatRatio } from '../utils/format'
import type { AiWorkInterval, ForegroundAppInterval } from '../domain/events'

const store = useAppStore()
const foregroundSegments = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground').map((event) => ({ start: event.start, end: event.end, color: '#58b982' })))
const aiSegments = computed(() => store.day.value.events.filter((event): event is AiWorkInterval => event.type === 'aiWork').map((event) => ({ start: event.start, end: event.end, color: event.accuracyLabel === 'precise' ? '#7a69dc' : '#a99ee9' })))
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 代理时间" subtitle="让工具默默工作，你专注于重要的事" />
    <div class="metrics-grid metrics-grid--four">
      <MetricCard label="有效代理工时" :value="formatDuration(store.day.value.aiEffective.value, true)" detail="代理工时可重叠累计" :icon="PhRobot" tone="violet" />
      <MetricCard label="工作覆盖时长" :value="formatDuration(store.day.value.aiCoverage.value, true)" detail="至少一个代理在工作" :icon="PhClock" tone="blue" />
      <MetricCard label="AI 杠杆率" :value="formatRatio(store.day.value.aiLeverage.value)" detail="有效工时 ÷ 前台交互" :icon="PhTrendUp" tone="orange" />
      <MetricCard label="最高并发数" :value="`${store.day.value.maxConcurrency.value ?? 0} 个`" detail="同一时刻有效代理" :icon="PhStack" tone="cyan" />
    </div>
    <article class="card ai-timeline-card">
      <div class="section-heading"><div><h2>AI 代理时间线</h2><p>与人的前台活动对比</p></div><div class="legend"><span class="green">人的前台活动</span><span class="violet">AI 代理工作</span></div></div>
      <div class="time-axis"><span>00:00</span><span>04:00</span><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span><span>24:00</span></div>
      <TimelineLane label="人的活动" :range="store.day.value.range" :segments="foregroundSegments" />
      <TimelineLane label="工具的工作" :range="store.day.value.range" :segments="aiSegments" />
      <div class="timeline-note"><span>前台交互</span><span>有效执行</span><span>并行重叠</span></div>
    </article>
    <article class="card tools-card">
      <div class="section-heading"><div><h2>AI 代理与工具</h2><p>点击工具查看统计依据</p></div></div>
      <div class="tools-table" role="table">
        <div class="tools-table__head" role="row"><span>工具</span><span>状态</span><span>有效工时</span><span>前台交互</span><span>任务</span><span>统计方式</span></div>
        <button v-for="tool in store.day.value.aiTools" :key="tool.toolId" type="button" class="tools-table__row" role="row" @click="store.openTool(tool.toolId)">
          <strong>{{ tool.toolName }}</strong><span class="status-text">运行记录</span><span>{{ formatDuration(tool.effectiveDuration, true) }}</span><span>{{ formatDuration(tool.foregroundDuration, true) }}</span><span>{{ tool.taskCount }}</span><span :class="['accuracy', tool.accuracyLabel]">{{ tool.accuracyLabel === 'precise' ? '精准统计' : '估算统计' }}</span>
        </button>
      </div>
    </article>
    <article class="ai-insight-card">
      <div><span>今日洞察</span><h2>AI 为你完成了 {{ formatDuration(store.day.value.aiEffective.value, true) }} 的有效工作</h2><p>覆盖现实时间 {{ formatDuration(store.day.value.aiCoverage.value, true) }}，并行增益 {{ formatRatio(store.day.value.parallelGain.value) }}。</p></div>
      <div class="gain-visual"><span style="height: 76%"></span><span style="height: 48%"></span></div>
    </article>
  </section>
</template>

