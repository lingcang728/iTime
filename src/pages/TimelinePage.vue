<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhCheckCircle,
  PhDesktop,
  PhInfo,
  PhMusicNotes,
  PhSparkle,
  PhSquaresFour,
  PhStack,
  PhX,
} from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import ActivityLane, { type ActivitySegment } from '../components/timeline/ActivityLane.vue'
import type {
  AiInteractionInterval,
  AiWorkInterval,
  DeviceStateInterval,
  ForegroundAppInterval,
  MediaPlaybackInterval,
  TimeEvent,
} from '../domain/events'
import { coalesceRangesBy } from '../domain/intervals'
import { comparisonLabel, metricDefinitions, metricInfo } from '../domain/metricDefinitions'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatPercent } from '../utils/format'
import { useNow } from '../composables/useNow'
import {
  isSelectedLocalDay,
  timelineNowPercent,
  timelineRange,
  timelineTicks,
  type TimelineRangeMode,
} from './timelineModel'

interface DurationPart { amount: string; unit?: string }

const store = useAppStore()
const notesOpen = ref(false)
const rangeMode = ref<TimelineRangeMode>('day')
const { nowMs } = useNow()
const isToday = computed(() => isSelectedLocalDay(store.state.selectedDate, nowMs.value))
const previousLabel = computed(() => isToday.value ? '昨日' : '前一日')
const nowPercent = computed(() => timelineNowPercent(
  store.state.selectedDate,
  displayRange.value,
  nowMs.value,
))
const deviceStyles = {
  active: { color: 'var(--timeline-device)', kind: 'attention', variant: 'solid', muted: false },
  idle: { color: 'var(--timeline-idle)', kind: 'waiting', variant: 'solid', muted: true },
  locked: { color: 'var(--timeline-away)', kind: 'waiting', variant: 'hatched', muted: true },
  sleep: { color: 'var(--timeline-away)', kind: 'waiting', variant: 'hatched', muted: true },
  unknown: { color: 'var(--timeline-away)', kind: 'waiting', variant: 'hatched', muted: true },
} as const
const deviceNames = { active: '活跃', idle: '空闲', locked: '离开', sleep: '离开', unknown: '未知' }
const byType = <T extends TimeEvent>(type: T['type']) => store.day.value.events.filter((event): event is T => event.type === type)
const displayRange = computed(() => timelineRange(store.day.value.range, rangeMode.value))
const axisTicks = computed(() => timelineTicks(rangeMode.value))
const rangeLabel = computed(() => rangeMode.value === 'day' ? '00:00 – 24:00' : '09:00 – 18:00')

const deviceSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<DeviceStateInterval>('device'),
  (event) => event.state,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, ...deviceStyles[event.state], title: deviceNames[event.state],
})))
const appSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<ForegroundAppInterval>('foreground'),
  (event) => event.appId,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: event.color || 'var(--timeline-app)', kind: 'other', title: event.appName, appId: event.appId,
})))
const aiSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<AiInteractionInterval>('aiInteraction'),
  (event) => event.toolId,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: 'var(--timeline-ai)', kind: 'interaction', title: event.toolName,
})))
// P3-59: aiWork（Agent 执行区间）由 provider 适配器真实产出，此前在时间线不可见。
const aiWorkSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<AiWorkInterval>('aiWork'),
  (event) => event.toolId,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: 'var(--accent-violet)', kind: 'agent', title: event.toolName,
})))
const mediaSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<MediaPlaybackInterval>('media'),
  (event) => `${event.appName}:${event.awayPlayback}`,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: 'var(--timeline-media)', kind: 'media', variant: event.awayPlayback ? 'hatched' : 'solid', muted: event.awayPlayback, title: event.appName,
})))
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const sourceLabel = computed(() => {
  if (store.state.activityDataStatus === 'degraded') return '部分本机记录'
  if (store.state.activityDataStatus === 'preview') return '预览数据'
  return '本机活动记录'
})
const sourceStateTitle = computed(() => {
  if (store.state.activityDataStatus === 'loading') return '正在读取活动记录'
  if (store.state.activityDataStatus === 'empty') return '当天暂无活动记录'
  return '活动记录读取失败'
})
const parallelRatio = computed(() => {
  const coverage = store.day.value.aiCoverage.value
  const overlap = store.day.value.parallelOverlap.value
  return coverage && overlap !== null ? overlap / coverage : null
})
const previousDay = computed(() => store.week.value.at(-2) ?? null)
const previousParallelRatio = computed(() => {
  const previous = previousDay.value
  const coverage = previous?.aiCoverage.value
  const overlap = previous?.parallelOverlap.value
  return typeof coverage === 'number' && coverage > 0 && typeof overlap === 'number'
    ? overlap / coverage
    : null
})
const foregroundComparison = computed(() => comparisonLabel(
  store.day.value.foregroundActivity.value,
  previousDay.value?.foregroundActivity.value ?? null,
  (value) => formatDuration(value, true),
  previousLabel.value,
))
const aiComparison = computed(() => comparisonLabel(
  store.day.value.aiInteraction.value,
  previousDay.value?.aiInteraction.value ?? null,
  (value) => formatDuration(value, true),
  previousLabel.value,
))
const parallelComparison = computed(() => comparisonLabel(
  parallelRatio.value,
  previousParallelRatio.value,
  (value) => `${Math.round(value * 100)} 个百分点`,
  previousLabel.value,
))
const foregroundTrend = computed(() => store.week.value.map((day) => day.foregroundActivity.value ?? 0))
const aiTrend = computed(() => store.week.value.map((day) => day.aiInteraction.value ?? 0))
const parallelTrend = computed(() => store.week.value.map((day) => {
  const coverage = day.aiCoverage.value
  const overlap = day.parallelOverlap.value
  return coverage && overlap !== null ? overlap / coverage : 0
}))

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  if (value < 3_600_000) return [{ amount: String(Math.round(value / 60_000)), unit: '分钟' }]
  return [{ amount: (value / 3_600_000).toFixed(1), unit: '小时' }]
}
</script>

<template>
  <section class="page timeline-page">
    <PageHeader title="时间线" subtitle="设备 · 应用 · AI · 媒体" />

    <div class="timeline-overview">
      <MetricCard :label="metricDefinitions.foregroundActivity.name" :value-parts="durationParts(store.day.value.foregroundActivity.value)" :detail="foregroundComparison" :icon="PhDesktop" visual="bars" :trend="foregroundTrend" :info="metricInfo('foregroundActivity')" />
      <MetricCard :label="metricDefinitions.aiInteraction.name" :value-parts="durationParts(store.day.value.aiInteraction.value)" :detail="aiComparison" :icon="PhSparkle" visual="bars" :trend="aiTrend" :info="metricInfo('aiInteraction')" />
      <MetricCard :label="metricDefinitions.providerParallelRatio.name" :value="formatPercent(parallelRatio)" :detail="parallelComparison" :icon="PhStack" visual="bars" :trend="parallelTrend" :info="metricInfo('providerParallelRatio')" />
    </div>

    <article class="full-timeline" aria-labelledby="activity-tracks-title">
      <header class="track-header">
        <div class="timeline-range-control">
          <div><strong id="activity-tracks-title">时间范围</strong><span>{{ rangeLabel }}</span></div>
          <div class="timeline-range-options" role="group" aria-label="时间线显示范围">
            <button type="button" :aria-pressed="rangeMode === 'day'" @click="rangeMode = 'day'">全天</button>
            <button type="button" :aria-pressed="rangeMode === 'work'" @click="rangeMode = 'work'">工作时段</button>
          </div>
        </div>
        <div class="track-actions">
          <div class="timeline-legend" aria-label="时间线颜色说明">
            <span><i class="device" />设备</span><span><i class="app" />应用</span><span><i class="ai" />AI</span><span v-if="aiWorkSegments.length"><i class="agent" />AI 执行</span><span v-if="mediaSegments.length"><i class="media" />媒体</span><span><i class="muted-hatch" />离开<span class="sr-only">设备非活跃</span></span>
          </div>
          <button type="button" class="timeline-info-button" aria-label="查看统计口径与轨道说明" aria-controls="timeline-notes" :aria-expanded="notesOpen" @click="notesOpen = !notesOpen" @keydown.escape="notesOpen = false"><PhInfo :size="17" aria-hidden="true" />说明</button>
          <Transition name="popover">
            <aside v-if="notesOpen" id="timeline-notes" class="timeline-popover" role="dialog" aria-label="统计口径与轨道说明">
              <button type="button" aria-label="关闭说明" @click="notesOpen = false"><PhX :size="14" aria-hidden="true" /></button>
              <div><span>口径</span><strong>重叠只计一次</strong><p>总覆盖 {{ formatDuration(store.day.value.totalDuration.value, true) }}</p></div>
              <div><span>轨道</span><strong>上下对齐 = 同时发生</strong><p>各轨独立，不重复计时。</p></div>
            </aside>
          </Transition>
        </div>
      </header>

      <template v-if="activityDataAvailable">
        <div class="timeline-axis">
          <span></span>
          <div class="timeline-axis__ticks"><span v-for="tick in axisTicks" :key="tick.label" :style="{ left: `${tick.percent}%` }">{{ tick.label }}</span></div>
        </div>
        <div class="timeline-tracks">
          <div v-if="nowPercent !== null" class="timeline-now-indicator" :style="{ left: `calc(var(--track-inset-start) + ${nowPercent / 100} * (100% - var(--track-inset-start) - var(--track-inset-end)))` }" aria-hidden="true" />
          <ActivityLane label="设备状态" :icon="PhDesktop" :range="displayRange" :segments="deviceSegments" />
          <ActivityLane label="前台应用" :icon="PhSquaresFour" :range="displayRange" :segments="appSegments" />
          <ActivityLane label="AI 前台活跃" :icon="PhSparkle" :range="displayRange" :segments="aiSegments" />
          <!-- P3-59: AI 执行/媒体轨道只在有数据时出现——原生侧不产出的恒空轨道与图例不常驻。 -->
          <ActivityLane v-if="aiWorkSegments.length" label="AI 执行" :icon="PhStack" :range="displayRange" :segments="aiWorkSegments" />
          <ActivityLane v-if="mediaSegments.length" label="媒体播放" :icon="PhMusicNotes" :range="displayRange" :segments="mediaSegments" />
        </div>
        <div class="timeline-explanation">
          <PhInfo :size="20" aria-hidden="true" />
          <div><strong>说明</strong><p>10 秒采样，同类合并；主刻度 1 小时，细格 15 分钟。仅前台计活跃。</p></div>
          <span><PhCheckCircle :size="15" aria-hidden="true" />{{ sourceLabel }}</span>
        </div>
      </template>
      <div v-else class="section-state timeline-source-state" :data-state="store.state.activityDataStatus">
        <strong>{{ sourceStateTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
      </div>
    </article>
  </section>
</template>

<style scoped>
.timeline-tracks {
  /* 与 .activity-lane 网格保持一致：18px 内边距 + 标签列 + 列间距 */
  --track-inset-start: 178px;
  --track-inset-end: 18px;
  position: relative;
}

@media (max-width: 780px) {
  .timeline-tracks {
    /* 与 timeline.css 同断点：lane 118px 标签列 + 8px 列间距 + 18px 内边距。 */
    --track-inset-start: 144px;
  }
}

.timeline-now-indicator {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: color-mix(in srgb, var(--text-primary) 55%, transparent);
  pointer-events: none;
  /* Behind the segments so the marker does not paint across their right edge. */
  z-index: 1;
}
</style>
