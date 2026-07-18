<script setup lang="ts">
import { computed } from 'vue'
import {
  PhArrowUpRight,
  PhChartLineUp,
  PhClockCounterClockwise,
  PhInfo,
  PhRobot,
  PhStack,
} from '@phosphor-icons/vue'
import AiActivityTimeline from '../components/AiActivityTimeline.vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import type { AiToolStatus, ForegroundAppInterval } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatRatio } from '../utils/format'

const store = useAppStore()
const foreground = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const statusLabels: Record<AiToolStatus, string> = { running: '采样中', completed: '有执行证据', waiting: '仅前台采样' }
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const executionAvailable = computed(() => activityDataAvailable.value && store.day.value.aiEffective.value !== null)
const pageSubtitle = computed(() => activityDataAvailable.value
  ? '区分可观察的前台活跃与需要 Provider 证据的后台执行'
  : store.state.activityDataStatus === 'loading' ? '正在读取本机活动记录' : '本机活动记录暂时不可用')
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取 AI 活动记录' : 'AI 活动记录暂不可用')

function activityValue(value: string): string {
  return activityDataAvailable.value ? value : '—'
}

function activityDetail(detail: string): string {
  return activityDataAvailable.value ? detail : store.state.activityDataMessage
}

const insight = computed(() => {
  const interaction = formatDuration(store.day.value.aiInteraction.value, true)
  if (!executionAvailable.value) {
    return {
      title: `记录到 ${interaction} 的 AI 前台活跃`,
      detail: '当前本机采集器只观察应用身份与设备活跃状态；没有 Agent 运行事件，因此不推算后台执行、并发或杠杆率。',
    }
  }
  const effective = formatDuration(store.day.value.aiEffective.value, true)
  const coverage = formatDuration(store.day.value.aiCoverage.value, true)
  const gain = store.day.value.parallelGain.value
  return {
    title: `当前口径记录到 ${effective} 的 AI 有效执行`,
    detail: gain === null
      ? `覆盖自然时间 ${coverage}；当前区间不足，暂不计算并行增益。`
      : `覆盖自然时间 ${coverage}；每 1 小时覆盖时间平均承载 ${gain.toFixed(1)} 小时工具执行。`,
  }
})
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 工具活动" :subtitle="pageSubtitle" />

    <div class="ai-metrics">
      <MetricCard
        label="AI 前台活跃"
        :value="activityValue(formatDuration(store.day.value.aiInteraction.value, true))"
        :detail="activityDetail('设备活跃且 AI 工具在前台')"
        :icon="PhRobot"
        tone="accent"
        info="来自 Windows 前台应用与设备活跃采样，不读取窗口标题、对话或输入内容。"
      />
      <MetricCard
        label="Provider 执行覆盖"
        :value="activityValue(formatDuration(store.day.value.aiCoverage.value, true))"
        :detail="activityDataAvailable ? executionAvailable ? '至少一个 Provider 报告执行的自然时长' : '当前数据源未提供 Agent 运行事件' : store.state.activityDataMessage"
        :icon="PhClockCounterClockwise"
        tone="neutral"
        info="只有接入能报告任务开始与结束的 Provider 时才计算；不会从前台窗口反推。"
      />
      <MetricCard
        label="AI 杠杆率"
        :value="activityValue(formatRatio(store.day.value.aiLeverage.value))"
        :detail="activityDataAvailable ? executionAvailable ? 'Provider 执行 ÷ 前台活跃' : '缺少 Provider 执行证据' : store.state.activityDataMessage"
        :icon="PhChartLineUp"
        tone="neutral"
        info="只表示执行时间与估算交互时间的比值，不等于生产力或产出质量。"
      />
      <MetricCard
        label="最高并发"
        :value="activityValue(store.day.value.maxConcurrency.value === null ? '—' : `${store.day.value.maxConcurrency.value} 个`)"
        :detail="activityDataAvailable ? executionAvailable ? '同一时刻有执行证据的工具数' : '缺少 Provider 执行证据' : store.state.activityDataMessage"
        :icon="PhStack"
        tone="neutral"
        info="当天任一时刻同时处于有效执行状态的工具峰值。"
      />
    </div>

    <div v-if="!activityDataAvailable" class="section-state ai-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <div v-else class="ai-workspace-grid">
      <section class="ai-panel ai-timeline-panel" aria-labelledby="ai-evidence-title">
        <div class="ai-section-heading">
          <div><span>区间视图</span><h2 id="ai-evidence-title">人的活动与 AI 工具证据</h2><p>实心执行区间仅来自 Provider；前台交互单独标记。</p></div>
          <div class="ai-section-hint"><PhInfo :size="14" />悬停或用键盘聚焦区间查看时间</div>
        </div>
        <AiActivityTimeline :range="store.day.value.range" :foreground="foreground" :tools="store.day.value.aiTools" />
      </section>

      <section class="ai-panel ai-tools-panel" aria-labelledby="ai-tools-title">
        <div class="ai-section-heading">
          <div><span>工具明细</span><h2 id="ai-tools-title">AI 工具</h2><p>同名工具已合并；没有 Provider 证据时仅展示可观察的前台活跃。</p></div>
        </div>
        <div v-if="store.day.value.aiTools.length" class="ai-tool-list" aria-label="AI 工具采样明细">
          <article v-for="tool in store.day.value.aiTools" :key="tool.toolId" class="ai-tool-item">
            <header>
              <span class="ai-tool-name"><ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="28" /><span><strong>{{ tool.toolName }}</strong><small>{{ tool.taskCount ? `${tool.taskCount} 组执行记录` : '仅前台观察' }}</small></span></span>
              <span :class="['ai-tool-status', `is-${tool.status}`]">{{ statusLabels[tool.status] }}</span>
            </header>
            <dl><div><dt>前台活跃</dt><dd>{{ formatDuration(tool.foregroundDuration, true) }}</dd></div><div><dt>Provider</dt><dd>{{ executionAvailable ? formatDuration(tool.effectiveDuration, true) : '暂无证据' }}</dd></div></dl>
            <footer><span class="ai-tool-confidence"><i :style="{ width: `${Math.round(tool.confidence * 100)}%` }"></i><b>置信度 {{ Math.round(tool.confidence * 100) }}%</b></span><button type="button" :aria-label="`查看 ${tool.toolName} 采样详情`" @click="store.openTool(tool.toolId)">详情<PhArrowUpRight :size="12" /></button></footer>
          </article>
        </div>
        <div v-else class="ai-tool-list__empty">当天尚未采集到 AI 工具活动。</div>
      </section>
    </div>

    <article v-if="activityDataAvailable" class="ai-insight">
      <div><span>今日洞察</span><h2>{{ insight.title }}</h2><p>{{ insight.detail }}</p></div>
      <dl><div><dt>前台活跃</dt><dd>{{ formatDuration(store.day.value.aiInteraction.value, true) }}</dd></div><div><dt>Provider 覆盖</dt><dd>{{ formatDuration(store.day.value.aiCoverage.value, true) }}</dd></div></dl>
    </article>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
