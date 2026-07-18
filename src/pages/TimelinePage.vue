<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhCalendarBlank,
  PhCaretDown,
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
  DeviceStateInterval,
  ForegroundAppInterval,
  MediaPlaybackInterval,
  TimeEvent,
} from '../domain/events'
import { coalesceRangesBy } from '../domain/intervals'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration, formatRatio } from '../utils/format'

interface DurationPart { amount: string; unit?: string }

const store = useAppStore()
const notesOpen = ref(false)
const deviceStyles = {
  active: { color: 'var(--accent-strong)', kind: 'attention', variant: 'solid', muted: false },
  idle: { color: 'var(--text-muted)', kind: 'waiting', variant: 'solid', muted: true },
  locked: { color: 'var(--text-muted)', kind: 'waiting', variant: 'hatched', muted: true },
  sleep: { color: 'var(--text-muted)', kind: 'waiting', variant: 'hatched', muted: true },
  unknown: { color: 'var(--text-muted)', kind: 'waiting', variant: 'hatched', muted: true },
} as const
const deviceNames = { active: '活跃', idle: '空闲', locked: '离开', sleep: '离开', unknown: '未知' }
const byType = <T extends TimeEvent>(type: T['type']) => store.day.value.events.filter((event): event is T => event.type === type)
const displayRange = computed(() => {
  const start = new Date(store.day.value.range.start)
  start.setHours(9, 0, 0, 0)
  const end = new Date(store.day.value.range.start)
  end.setHours(18, 0, 0, 0)
  return { start: start.getTime(), end: end.getTime() }
})

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
  start: event.start, end: event.end, color: 'var(--accent-strong)', kind: 'other', title: event.appName,
})))
const aiSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<AiInteractionInterval>('aiInteraction'),
  (event) => event.toolId,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: 'var(--accent-strong)', kind: 'interaction', title: event.toolName,
})))
const mediaSegments = computed<ActivitySegment[]>(() => coalesceRangesBy(
  byType<MediaPlaybackInterval>('media'),
  (event) => `${event.appName}:${event.awayPlayback}`,
  20_000,
).map((event) => ({
  start: event.start, end: event.end, color: 'var(--text-secondary)', kind: 'media', variant: event.awayPlayback ? 'hatched' : 'solid', muted: event.awayPlayback, title: event.appName,
})))
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const sourceLabel = computed(() => ({ ready: '本机活动记录', degraded: '部分本机记录', preview: '预览数据', loading: '正在读取', unavailable: '暂不可用' }[store.state.activityDataStatus]))
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取活动记录' : '活动记录暂不可用')
const parallelRatio = computed(() => {
  const coverage = store.day.value.aiCoverage.value
  const overlap = store.day.value.parallelOverlap.value
  return coverage && overlap !== null ? overlap / coverage : null
})

function durationParts(value: number | null): DurationPart[] {
  if (value === null) return [{ amount: '—', unit: '暂无数据' }]
  if (value < 3_600_000) return [{ amount: String(Math.round(value / 60_000)), unit: '分钟' }]
  return [{ amount: (value / 3_600_000).toFixed(1), unit: '小时' }]
}
</script>

<template>
  <section class="page timeline-page">
    <PageHeader title="时间线" subtitle="回顾你的设备活动、应用使用、AI 前台与媒体播放的时间分布与切换轨迹。" />

    <div class="timeline-overview">
      <MetricCard label="前台活动" :value-parts="durationParts(store.day.value.foregroundActivity.value)" detail="较昨日  +1.2 小时" :icon="PhDesktop" visual="bars" />
      <MetricCard label="AI 前台活跃" :value-parts="durationParts(store.day.value.aiInteraction.value)" detail="较昨日  +0.4 小时" :icon="PhSparkle" visual="bars" />
      <MetricCard label="Provider 并行" :value="formatRatio(parallelRatio)" detail="较昨日  +6%" :icon="PhStack" visual="bars" />
    </div>

    <article class="full-timeline" aria-labelledby="activity-tracks-title">
      <header class="track-header">
        <button type="button" class="timeline-range-button"><PhCalendarBlank :size="18" /><strong id="activity-tracks-title">时间范围</strong><span>09:00 – 18:00</span><PhCaretDown :size="13" /></button>
        <div class="track-actions">
          <div class="timeline-legend" aria-label="时间线颜色说明">
            <span><i class="sage" />活跃</span><span><i class="neutral" />空闲<span class="sr-only">设备非活跃</span></span><span><i class="muted-hatch" />离开</span>
          </div>
          <button type="button" class="timeline-info-button" aria-label="查看统计口径与轨道说明" aria-controls="timeline-notes" :aria-expanded="notesOpen" @click="notesOpen = !notesOpen" @keydown.escape="notesOpen = false"><PhInfo :size="17" />说明</button>
          <Transition name="popover">
            <aside v-if="notesOpen" id="timeline-notes" class="timeline-popover" role="dialog" aria-label="统计口径与轨道说明">
              <button type="button" aria-label="关闭说明" @click="notesOpen = false"><PhX :size="14" /></button>
              <div><span>统计口径</span><strong>总覆盖按自然时间并集计算</strong><p>前台活动与 Provider 执行重叠时只计一次；当前总覆盖为 {{ formatDuration(store.day.value.totalDuration.value, true) }}。</p></div>
              <div><span>轨道说明</span><strong>上下对齐表示同时发生</strong><p>设备、应用、AI 与媒体各自保留来源，并行时段不会重复计入总时长。</p></div>
            </aside>
          </Transition>
        </div>
      </header>

      <template v-if="activityDataAvailable">
        <div class="timeline-axis">
          <span></span>
          <div class="timeline-axis__ticks"><span v-for="hour in 10" :key="hour" :style="{ left: `${(hour - 1) / 9 * 100}%` }">{{ String(hour + 8).padStart(2, '0') }}:00</span></div>
        </div>
        <div class="timeline-tracks">
          <ActivityLane label="设备状态" :icon="PhDesktop" :range="displayRange" :segments="deviceSegments" />
          <ActivityLane label="前台应用" :icon="PhSquaresFour" :range="displayRange" :segments="appSegments" />
          <ActivityLane label="AI 前台" :icon="PhSparkle" :range="displayRange" :segments="aiSegments" />
          <ActivityLane label="媒体播放" :icon="PhMusicNotes" :range="displayRange" :segments="mediaSegments" />
        </div>
        <div class="timeline-explanation">
          <PhInfo :size="20" />
          <div><strong>说明</strong><p>时间线按 10 秒采样，并把连续同类记录合并成区间；主刻度为 1 小时，细网格为 15 分钟。<br>前台应用与 AI 前台仅在窗口处于前台时计为活跃时长。</p></div>
          <span><PhCheckCircle :size="15" />{{ sourceLabel }}</span>
        </div>
      </template>
      <div v-else class="section-state timeline-source-state" :data-state="store.state.activityDataStatus">
        <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
      </div>
    </article>
  </section>
</template>
