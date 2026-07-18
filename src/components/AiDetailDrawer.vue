<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { PhCheckCircle, PhWarningCircle, PhX } from '@phosphor-icons/vue'
import type { AiToolStatus } from '../domain/events'
import ApplicationIcon from './ApplicationIcon.vue'
import { useAppStore } from '../stores/appStore'
import { formatClock, formatDuration } from '../utils/format'

type IntervalTone = 'work' | 'interaction' | 'wait'

interface IntervalRow {
  id: string
  start: number
  end: number
  label: string
  tone: IntervalTone
}

const store = useAppStore()
const drawer = ref<HTMLElement | null>(null)
const tool = computed(() => store.selectedTool.value)
const executionAvailable = computed(() => store.day.value.aiEffective.value !== null)
const statusLabels: Record<AiToolStatus, string> = { running: '采样中', completed: '有执行证据', waiting: '仅前台采样' }
let returnFocus: HTMLElement | null = null

const intervalRows = computed<IntervalRow[]>(() => {
  if (!tool.value) return []
  return [
    ...tool.value.workIntervals.map((item) => ({ id: `work:${item.id}`, start: item.start, end: item.end, label: 'Provider 执行', tone: 'work' as const })),
    ...tool.value.interactionIntervals.map((item) => ({ id: `interaction:${item.id}`, start: item.start, end: item.end, label: '前台活跃', tone: 'interaction' as const })),
    ...tool.value.waitIntervals.map((item, index) => ({ id: `wait-${index}`, start: item.start, end: item.end, label: '静默等待', tone: 'wait' as const })),
  ].sort((a, b) => a.start - b.start)
})

const definitions = [
  ['Provider 执行', '仅统计能报告任务开始与结束的 Provider 区间；当前来源只观察前台活动时保持不可用。'],
  ['前台活跃', '设备处于活跃状态且前台进程匹配 AI 工具；不代表持续输入，也不读取提示词、对话或文件内容。'],
  ['静默等待', '工具处于等待、暂停等状态的区间，不计入有效执行。'],
  ['并行重叠', '工具有效执行与人的前台活动在时间上相交；只说明同时发生，不表示因果关系。'],
  ['检测置信度', '表示采样证据对活动归属的支持程度，不是工具的“知性度”、产出质量或能力评分。'],
] as const

function onKeydown(event: KeyboardEvent): void {
  if (!store.state.detailDrawerOpen) return
  if (event.key === 'Escape') {
    event.preventDefault()
    store.closeTool()
    return
  }
  if (event.key !== 'Tab' || !drawer.value) return
  const focusable = [...drawer.value.querySelectorAll<HTMLElement>('button, [href], input, [tabindex]:not([tabindex="-1"])')]
    .filter((element) => !element.hasAttribute('disabled'))
  if (!focusable.length) return
  const first = focusable[0]!
  const last = focusable[focusable.length - 1]!
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first.focus()
  }
}

watch(() => store.state.detailDrawerOpen, (open) => {
  if (open) {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    void nextTick(() => drawer.value?.querySelector<HTMLElement>('button')?.focus())
  } else {
    void nextTick(() => returnFocus?.focus())
  }
})
onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Transition name="drawer">
    <div v-if="store.state.detailDrawerOpen" class="ai-drawer-backdrop" @click.self="store.closeTool()">
      <aside ref="drawer" class="ai-drawer" role="dialog" aria-modal="true" aria-labelledby="ai-detail-title" tabindex="-1">
        <button class="ai-drawer__close" type="button" aria-label="关闭详情" @click="store.closeTool()"><PhX :size="18" /></button>
        <template v-if="tool">
          <header class="ai-drawer__header">
            <ApplicationIcon :icon-key="tool.iconKey" :app-name="tool.toolName" :size="38" />
            <div><span>本机采样详情</span><h2 id="ai-detail-title">{{ tool.toolName }}</h2></div>
            <em :class="`is-${tool.status}`">{{ statusLabels[tool.status] }}</em>
          </header>

          <p class="ai-drawer__privacy">数据来自本机活动事件的时间匹配。iTime 不读取提示词、对话、按键内容或文件正文。</p>

          <dl class="ai-drawer__metrics">
            <div><dt>Provider 执行</dt><dd>{{ executionAvailable ? formatDuration(tool.effectiveDuration, true) : '—' }}</dd><small>{{ executionAvailable ? `${tool.taskCount} 组执行记录` : '当前来源未提供' }}</small></div>
            <div><dt>前台活跃</dt><dd>{{ formatDuration(tool.foregroundDuration, true) }}</dd><small>设备活跃且工具在前台</small></div>
            <div><dt>静默等待</dt><dd>{{ executionAvailable ? formatDuration(tool.silentWaitDuration, true) : '—' }}</dd><small>{{ executionAvailable ? '不计入 Provider 执行' : '当前来源未提供' }}</small></div>
            <div><dt>并行重叠</dt><dd>{{ executionAvailable ? formatDuration(tool.parallelOverlapDuration, true) : '—' }}</dd><small>{{ executionAvailable ? '与人的活动同时发生' : '当前来源未提供' }}</small></div>
          </dl>

          <section class="ai-drawer__section">
            <div class="ai-drawer__section-title"><div><span>运行区间</span><h3>当天采样记录</h3></div><strong>{{ intervalRows.length }} 条</strong></div>
            <div v-if="intervalRows.length" class="ai-drawer__intervals">
              <div v-for="item in intervalRows" :key="item.id" class="ai-drawer__interval">
                <i :class="`is-${item.tone}`"></i><span>{{ formatClock(item.start) }}–{{ formatClock(item.end) }}</span><strong>{{ item.label }}</strong>
              </div>
            </div>
            <p v-else class="ai-drawer__empty">当天没有可显示的区间。</p>
          </section>

          <section class="ai-drawer__section">
            <div class="ai-drawer__section-title"><div><span>指标口径</span><h3>这些数字分别代表什么</h3></div></div>
            <dl class="ai-drawer__definitions">
              <div v-for="definition in definitions" :key="definition[0]"><dt>{{ definition[0] }}</dt><dd>{{ definition[1] }}</dd></div>
            </dl>
          </section>

          <section class="ai-drawer__section">
            <div class="ai-drawer__section-title"><div><span>检测依据</span><h3>活动为何归到这个工具</h3></div></div>
            <ul class="ai-drawer__evidence"><li v-for="basis in tool.detectionBasis" :key="basis"><PhCheckCircle :size="14" />{{ basis }}</li></ul>
            <div class="ai-drawer__confidence">
              <span>检测置信度 <small>{{ tool.accuracyLabel === 'precise' ? '证据完整' : '包含估算' }}</small></span>
              <div><i :style="{ width: `${Math.round(tool.confidence * 100)}%` }"></i></div>
              <strong>{{ Math.round(tool.confidence * 100) }}%</strong>
            </div>
          </section>

          <section v-if="tool.pendingRecords.length" class="ai-drawer__warning">
            <PhWarningCircle :size="18" /><div><h3>有 {{ tool.pendingRecords.length }} 条区间需要确认</h3><p>证据不足的记录仍会展示，但会保留“估算”标记。</p></div>
          </section>
        </template>
      </aside>
    </div>
  </Transition>
</template>

<style scoped src="./ai-detail-drawer.css"></style>
