<script setup lang="ts">
import { computed } from 'vue'
import { PhArrowUpRight, PhInfo } from '@phosphor-icons/vue'
import AgentMetricCard from '../components/AgentMetricCard.vue'
import AiActivityTimeline from '../components/AiActivityTimeline.vue'
import ApplicationIcon from '../components/ApplicationIcon.vue'
import PageHeader from '../components/PageHeader.vue'
import { uiIcons } from '../data/uiIcons'
import type { AiToolStatus, ForegroundAppInterval } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { formatDuration, formatRatio } from '../utils/format'

const store = useAppStore()
const foreground = computed(() => store.day.value.events.filter((event): event is ForegroundAppInterval => event.type === 'foreground'))
const statusLabels: Record<AiToolStatus, string> = { running: '采样中', completed: '有执行证据', waiting: '仅前台采样' }
const executionAvailable = computed(() => store.day.value.aiEffective.value !== null)

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
    <PageHeader title="AI 工具活动" subtitle="区分可观察的前台活跃与需要 Provider 证据的后台执行" />

    <div class="ai-metrics">
      <AgentMetricCard
        label="AI 前台活跃"
        :value="formatDuration(store.day.value.aiInteraction.value, true)"
        detail="设备活跃且 AI 工具在前台"
        :icon-src="uiIcons.aiEffective"
        tone="violet"
        info="来自 Windows 前台应用与设备活跃采样，不读取窗口标题、对话或输入内容。"
      />
      <AgentMetricCard
        label="Provider 执行覆盖"
        :value="formatDuration(store.day.value.aiCoverage.value, true)"
        :detail="executionAvailable ? '至少一个 Provider 报告执行的自然时长' : '当前数据源未提供 Agent 运行事件'"
        :icon-src="uiIcons.aiCoverage"
        tone="blue"
        info="只有接入能报告任务开始与结束的 Provider 时才计算；不会从前台窗口反推。"
      />
      <AgentMetricCard
        label="AI 杠杆率"
        :value="formatRatio(store.day.value.aiLeverage.value)"
        :detail="executionAvailable ? 'Provider 执行 ÷ 前台活跃' : '缺少 Provider 执行证据'"
        :icon-src="uiIcons.aiLeverage"
        tone="orange"
        info="只表示执行时间与估算交互时间的比值，不等于生产力或产出质量。"
      />
      <AgentMetricCard
        label="最高并发"
        :value="store.day.value.maxConcurrency.value === null ? '—' : `${store.day.value.maxConcurrency.value} 个`"
        :detail="executionAvailable ? '同一时刻有执行证据的工具数' : '缺少 Provider 执行证据'"
        :icon-src="uiIcons.aiConcurrency"
        tone="cyan"
        info="当天任一时刻同时处于有效执行状态的工具峰值。"
      />
    </div>

    <article class="ai-panel ai-timeline-panel">
      <div class="ai-section-heading">
        <div><span>区间视图</span><h2>人的活动与 AI 工具证据</h2><p>实心执行区间仅来自 Provider；前台交互单独标记。</p></div>
        <div class="ai-section-hint"><PhInfo :size="14" />悬停或用键盘聚焦区间查看时间</div>
      </div>
      <AiActivityTimeline :range="store.day.value.range" :foreground="foreground" :tools="store.day.value.aiTools" />
    </article>

    <article class="ai-panel ai-tools-panel">
      <div class="ai-section-heading">
        <div><span>工具明细</span><h2>AI 工具</h2><p>前台活跃来自进程身份匹配；没有 Provider 证据时，执行相关列显示为不可用。</p></div>
      </div>
      <div class="ai-tool-table" role="table" aria-label="AI 工具采样明细">
        <div class="ai-tool-table__head" role="row">
          <span role="columnheader">工具</span><span role="columnheader">证据</span><span role="columnheader">Provider 执行</span><span role="columnheader">前台活跃</span><span role="columnheader">静默等待</span><span role="columnheader">并行重叠</span><span role="columnheader">置信度</span><span role="columnheader">操作</span>
        </div>
        <div v-if="!store.day.value.aiTools.length" class="ai-tool-table__empty">当天尚未采集到 AI 工具活动。</div>
        <div v-for="tool in store.day.value.aiTools" :key="tool.toolId" class="ai-tool-table__row" role="row">
          <span class="ai-tool-name" role="cell"><ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="24" /><span><strong>{{ tool.toolName }}</strong><small>{{ tool.taskCount ? `${tool.taskCount} 组执行记录` : '没有执行记录' }}</small></span></span>
          <span :class="['ai-tool-status', `is-${tool.status}`]" role="cell">{{ statusLabels[tool.status] }}</span>
          <span role="cell">{{ executionAvailable ? formatDuration(tool.effectiveDuration, true) : '—' }}</span>
          <span role="cell">{{ formatDuration(tool.foregroundDuration, true) }}</span>
          <span role="cell">{{ executionAvailable ? formatDuration(tool.silentWaitDuration, true) : '—' }}</span>
          <span role="cell">{{ executionAvailable ? formatDuration(tool.parallelOverlapDuration, true) : '—' }}</span>
          <span class="ai-tool-confidence" role="cell"><i :style="{ width: `${Math.round(tool.confidence * 100)}%` }"></i><b>{{ Math.round(tool.confidence * 100) }}%</b></span>
          <button role="cell" type="button" :aria-label="`查看 ${tool.toolName} 采样详情`" @click="store.openTool(tool.toolId)">详情<PhArrowUpRight :size="12" /></button>
        </div>
      </div>
    </article>

    <article class="ai-insight">
      <div><span>今日洞察</span><h2>{{ insight.title }}</h2><p>{{ insight.detail }}</p></div>
      <dl><div><dt>前台活跃</dt><dd>{{ formatDuration(store.day.value.aiInteraction.value, true) }}</dd></div><div><dt>Provider 覆盖</dt><dd>{{ formatDuration(store.day.value.aiCoverage.value, true) }}</dd></div></dl>
    </article>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
