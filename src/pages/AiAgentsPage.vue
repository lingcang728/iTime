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
const statusLabels: Record<AiToolStatus, string> = { running: '采样中', completed: '已记录', waiting: '等待中' }

const insight = computed(() => {
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
    <PageHeader title="AI 代理时间" subtitle="核对工具活动、前台交互与并行发生的本机采样区间" />

    <div class="ai-metrics">
      <AgentMetricCard
        label="有效执行"
        :value="formatDuration(store.day.value.aiEffective.value, true)"
        detail="各工具执行区间分别累计"
        :icon-src="uiIcons.aiEffective"
        tone="violet"
        info="当前数据源归为 AI 工作的区间总和；前台采样来源仍属于估算。"
      />
      <AgentMetricCard
        label="工作覆盖"
        :value="formatDuration(store.day.value.aiCoverage.value, true)"
        detail="至少一个工具在工作的自然时长"
        :icon-src="uiIcons.aiCoverage"
        tone="blue"
        info="把重叠的工具执行区间合并后得到的自然经过时间。"
      />
      <AgentMetricCard
        label="AI 杠杆率"
        :value="formatRatio(store.day.value.aiLeverage.value)"
        detail="有效执行 ÷ 前台交互（估算）"
        :icon-src="uiIcons.aiLeverage"
        tone="orange"
        info="只表示执行时间与估算交互时间的比值，不等于生产力或产出质量。"
      />
      <AgentMetricCard
        label="最高并发"
        :value="`${store.day.value.maxConcurrency.value ?? 0} 个`"
        detail="同一时刻有效执行的工具数"
        :icon-src="uiIcons.aiConcurrency"
        tone="cyan"
        info="当天任一时刻同时处于有效执行状态的工具峰值。"
      />
    </div>

    <article class="ai-panel ai-timeline-panel">
      <div class="ai-section-heading">
        <div><span>区间视图</span><h2>人的活动与工具执行</h2><p>每一行对应当前数据源中的实际采样区间。</p></div>
        <div class="ai-section-hint"><PhInfo :size="14" />悬停或用键盘聚焦区间查看时间</div>
      </div>
      <AiActivityTimeline :range="store.day.value.range" :foreground="foreground" :tools="store.day.value.aiTools" />
    </article>

    <article class="ai-panel ai-tools-panel">
      <div class="ai-section-heading">
        <div><span>工具明细</span><h2>AI 工具</h2><p>前台交互来自窗口与工具身份匹配，是估算值；不会读取交互内容。</p></div>
      </div>
      <div class="ai-tool-table" role="table" aria-label="AI 工具采样明细">
        <div class="ai-tool-table__head" role="row">
          <span>工具</span><span>状态</span><span>有效执行</span><span>前台交互</span><span>静默等待</span><span>并行重叠</span><span>置信度</span><span></span>
        </div>
        <div v-if="!store.day.value.aiTools.length" class="ai-tool-table__empty">当天尚未采集到 AI 工具活动。</div>
        <div v-for="tool in store.day.value.aiTools" :key="tool.toolId" class="ai-tool-table__row" role="row">
          <span class="ai-tool-name"><ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="24" /><span><strong>{{ tool.toolName }}</strong><small>{{ tool.taskCount }} 组执行记录</small></span></span>
          <span :class="['ai-tool-status', `is-${tool.status}`]">{{ statusLabels[tool.status] }}</span>
          <span>{{ formatDuration(tool.effectiveDuration, true) }}</span>
          <span>{{ formatDuration(tool.foregroundDuration, true) }}</span>
          <span>{{ formatDuration(tool.silentWaitDuration, true) }}</span>
          <span>{{ formatDuration(tool.parallelOverlapDuration, true) }}</span>
          <span class="ai-tool-confidence"><i :style="{ width: `${Math.round(tool.confidence * 100)}%` }"></i><b>{{ Math.round(tool.confidence * 100) }}%</b></span>
          <button type="button" :aria-label="`查看 ${tool.toolName} 采样详情`" @click="store.openTool(tool.toolId)">详情<PhArrowUpRight :size="12" /></button>
        </div>
      </div>
    </article>

    <article class="ai-insight">
      <div><span>今日洞察</span><h2>{{ insight.title }}</h2><p>{{ insight.detail }}</p></div>
      <dl><div><dt>有效执行</dt><dd>{{ formatDuration(store.day.value.aiEffective.value, true) }}</dd></div><div><dt>覆盖时间</dt><dd>{{ formatDuration(store.day.value.aiCoverage.value, true) }}</dd></div></dl>
    </article>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
