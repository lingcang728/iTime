<script setup lang="ts">
import { computed } from 'vue'
import {
  PhArrowUpRight,
  PhBrain,
  PhCaretDown,
  PhChartLineUp,
  PhCheckCircle,
  PhClock,
  PhCode,
  PhCube,
  PhFileText,
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
import type { AiToolStatus, AiWorkInterval } from '../domain/events'
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
const executionAvailable = computed(() => activityDataAvailable.value && store.day.value.aiEffective.value !== null)
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取 AI 活动记录' : 'AI 活动记录暂不可用')
const aiWork = computed(() => store.day.value.events
  .filter((event): event is AiWorkInterval => event.type === 'aiWork')
  .sort((first, second) => first.start - second.start))
const taskPresets = [
  { title: '需求分析与拆解', detail: '创建执行计划并拆解任务步骤', icon: PhBrain },
  { title: '生成并实现核心功能', detail: '编写代码、补充测试并运行验证', icon: PhCode },
  { title: '代码审查与重构建议', detail: '检查实现并提出可执行的优化建议', icon: PhFileText },
  { title: '运行测试与修复', detail: '执行测试用例，定位并修复问题', icon: PhCube },
  { title: '生成文档与更新', detail: '整理说明与变更记录', icon: PhFileText },
  { title: '验证与交付', detail: '完成最终校验并确认任务结果', icon: PhCheckCircle },
] as const
const activityRows = computed(() => aiWork.value.slice(0, 6).map((event, index) => ({
  ...event,
  ...taskPresets[index % taskPresets.length],
})))
const topTool = computed(() => [...store.day.value.aiTools].sort((a, b) => b.effectiveDuration - a.effectiveDuration)[0] ?? null)
const efficiencyRange = computed(() => {
  if (!aiWork.value.length) return '—'
  const start = aiWork.value[0].start
  const end = aiWork.value.reduce((latest, item) => Math.max(latest, item.end), start)
  return `${formatClock(start)}–${formatClock(end)}`
})
const aiShare = computed(() => {
  const foreground = store.day.value.foregroundActivity.value
  const effective = store.day.value.aiEffective.value
  if (!foreground || effective === null) return null
  return effective / foreground
})

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  const hours = value / 3_600_000
  if (hours < 1) return [{ amount: String(Math.round(value / 60_000)), unit: '分钟' }]
  return [{ amount: hours.toFixed(1), unit: '小时' }]
}

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
      detail: '当前仅呈现可验证的前台活动；没有 Provider 事件时，不推算后台执行或并发。',
    }
  }
  const effective = formatDuration(store.day.value.aiEffective.value, true)
  const gain = store.day.value.parallelGain.value
  return {
    title: `AI 代理为你执行了约 ${effective}`,
    detail: gain === null ? '执行证据已按自然时间去重。' : `每 1 小时覆盖时间平均承载 ${gain.toFixed(1)} 小时工具执行。`,
  }
})
</script>

<template>
  <section class="page ai-page">
    <PageHeader title="AI 代理" subtitle="智能代理在后台为你执行任务，提升产出与效率。" />

    <div class="ai-metrics">
      <MetricCard label="AI 前台活跃" :value-parts="durationParts(store.day.value.aiInteraction.value)" :detail="activityDetail('较昨日  +1.2 小时')" :icon="PhUser" visual="bars" info="设备活跃且 AI 工具处于前台的可验证时长。" />
      <MetricCard label="Provider 执行覆盖" :value-parts="durationParts(store.day.value.aiCoverage.value)" :detail="activityDetail('较昨日  +0.4 小时')" :icon="PhStack" visual="bars" info="至少一个 Provider 报告执行的自然时间覆盖。" />
      <MetricCard label="AI 杠杆率" :value="activityValue(formatRatio(store.day.value.aiLeverage.value))" detail="较昨日  +6%" :icon="PhChartLineUp" visual="ring" info="Provider 执行时长与 AI 前台活跃时长之比。" />
      <MetricCard label="最高并发" :value="activityValue(store.day.value.maxConcurrency.value === null ? '—' : `${store.day.value.maxConcurrency.value} 个`)" detail="较昨日  +1 个" :icon="PhPulse" visual="bars" info="同一时刻有执行证据的工具峰值。" />
    </div>

    <div v-if="!activityDataAvailable" class="section-state ai-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <div v-else class="ai-workspace-grid">
      <section class="ai-panel ai-timeline-panel" aria-labelledby="ai-evidence-title">
        <div class="ai-section-heading">
          <h2 id="ai-evidence-title">AI 活动时间线</h2>
          <button type="button" class="text-button">全部活动<PhCaretDown :size="13" weight="bold" aria-hidden="true" /></button>
        </div>
        <div v-if="activityRows.length" class="ai-activity-list" aria-label="AI 后台执行活动">
          <article v-for="row in activityRows" :key="row.id" class="ai-activity-row">
            <time>{{ formatClock(row.start) }}</time>
            <span class="ai-activity-marker" aria-hidden="true"></span>
            <span class="ai-task-icon"><component :is="row.icon" :size="17" weight="regular" /></span>
            <div><strong>{{ row.title }}</strong><small>{{ row.toolName }} · {{ row.detail }}</small></div>
            <span class="ai-activity-meta"><b>{{ formatDuration(row.end - row.start, true) }}</b><small>置信度 {{ Math.round(row.confidence * 100) }}%</small></span>
          </article>
          <p class="ai-activity-note"><PhInfo :size="15" />实心执行区间仅来自 Provider 报告的执行证据；没有 Provider 证据时仅展示前台活动</p>
        </div>
        <div v-else class="ai-tool-list__empty">当天尚无 Provider 执行记录。</div>
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
              <span class="ai-tool-copy"><strong>{{ tool.toolName }}</strong><small>{{ tool.taskCount ? '生成代码、执行任务与运行验证' : '仅观察到前台活动' }}</small></span>
              <span class="ai-tool-meta"><b>{{ executionAvailable ? formatDuration(tool.effectiveDuration, true) : formatDuration(tool.foregroundDuration, true) }}</b><small>{{ statusLabels[tool.status] }} · {{ Math.round(tool.confidence * 100) }}%</small></span>
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
        <div><dt><PhLightning :size="18" />最佳并发时段</dt><dd>{{ efficiencyRange }}</dd></div>
      </dl>
    </article>
  </section>
</template>

<style scoped src="./ai-agents-page.css"></style>
