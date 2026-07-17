<script setup lang="ts">
import { computed } from 'vue'
import { PhActivity, PhCheckCircle, PhClockCounterClockwise, PhStack } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import ActivityLane, { type ActivitySegment } from '../components/timeline/ActivityLane.vue'
import type {
  AiInteractionInterval,
  DeviceStateInterval,
  ForegroundAppInterval,
  MediaPlaybackInterval,
  TimeEvent,
  VoiceInputInterval,
} from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { formatDuration } from '../utils/format'

const store = useAppStore()
const deviceColors = { active: '#55ad78', idle: '#d29a45', locked: '#7e8ea3', sleep: '#8e83b8', unknown: '#a3a8ae' }
const deviceNames = { active: '设备活跃', idle: '暂时空闲', locked: '设备锁定', sleep: '设备休眠', unknown: '状态未知' }
const byType = <T extends TimeEvent>(type: T['type']) => store.day.value.events.filter((event): event is T => event.type === type)

const deviceSegments = computed<ActivitySegment[]>(() => byType<DeviceStateInterval>('device').map((event) => ({
  start: event.start,
  end: event.end,
  color: deviceColors[event.state],
  muted: event.state !== 'active',
  kind: event.state === 'active' ? 'attention' : 'waiting',
  title: deviceNames[event.state],
})))
const appSegments = computed<ActivitySegment[]>(() => byType<ForegroundAppInterval>('foreground').map((event) => ({
  start: event.start,
  end: event.end,
  color: event.color || '#4c79e8',
  kind: 'other',
  title: event.appName,
})))
const aiSegments = computed<ActivitySegment[]>(() => byType<AiInteractionInterval>('aiInteraction').map((event) => ({
  start: event.start,
  end: event.end,
  color: '#7664d8',
  kind: 'interaction',
  variant: 'hatched',
  title: `${event.toolName}（前台活跃）`,
})))
const voiceSegments = computed<ActivitySegment[]>(() => byType<VoiceInputInterval>('voice').map((event) => ({
  start: event.start,
  end: event.end,
  color: '#4aaba8',
  kind: 'interaction',
  title: `${event.toolName} 语音输入`,
})))
const mediaSegments = computed<ActivitySegment[]>(() => byType<MediaPlaybackInterval>('media').map((event) => ({
  start: event.start,
  end: event.end,
  color: '#d98255',
  kind: 'media',
  variant: event.awayPlayback ? 'outline' : 'solid',
  title: `${event.appName}${event.awayPlayback ? '（离开播放）' : ''}`,
})))
const pageSubtitle = computed(() => store.state.activityDataStatus === 'ready'
  ? '用同一日活动记录核对设备、前台应用与 AI 前台活跃'
  : '展示设备、前台应用与 AI 前台活跃在一天中的关系')
const sourceLabel = computed(() => store.state.activityDataStatus === 'ready' ? '本机活动记录' : '预览数据')
</script>

<template>
  <section class="page timeline-page">
    <PageHeader title="今日时间线" :subtitle="pageSubtitle" />

    <div class="timeline-overview">
      <article class="timeline-stat stat-green"><span>前台活动</span><strong>{{ formatDuration(store.day.value.foregroundActivity.value, true) }}</strong><small>设备活跃且有前台应用</small></article>
      <article class="timeline-stat stat-violet"><span>AI 前台活跃</span><strong>{{ formatDuration(store.day.value.aiInteraction.value, true) }}</strong><small>设备活跃且 AI 工具处于前台</small></article>
      <article class="timeline-stat stat-blue"><span>Provider 并行</span><strong>{{ formatDuration(store.day.value.parallelOverlap.value, true) }}</strong><small>仅有执行事件时计算</small></article>
    </div>

    <article class="card full-timeline" aria-labelledby="activity-tracks-title">
      <header class="track-header">
        <div><h2 id="activity-tracks-title">一天的活动分布</h2><p>五条轨道共用同一时间刻度；悬停或聚焦色块查看区间</p></div>
        <span><PhCheckCircle :size="15" weight="fill" />{{ sourceLabel }}</span>
      </header>
      <div class="timeline-axis"><span></span><span>00:00</span><span>04:00</span><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span><span>24:00</span></div>
      <div class="timeline-tracks">
        <ActivityLane label="设备状态" :range="store.day.value.range" :segments="deviceSegments" />
        <ActivityLane label="前台应用" :range="store.day.value.range" :segments="appSegments" />
        <ActivityLane label="AI 前台" :range="store.day.value.range" :segments="aiSegments" />
        <ActivityLane label="语音输入" :range="store.day.value.range" :segments="voiceSegments" />
        <ActivityLane label="媒体播放" :range="store.day.value.range" :segments="mediaSegments" />
      </div>
      <div class="timeline-legend" aria-label="时间线颜色说明">
        <span><i class="green" />设备活跃</span><span><i class="idle" />设备空闲</span><span><i class="unknown" />状态未知</span><span><i class="blue" />前台应用</span><span><i class="violet" />AI 前台</span><span><i class="cyan" />语音输入</span><span><i class="orange" />媒体播放</span>
      </div>
    </article>

    <div class="timeline-detail-grid">
      <article class="card event-summary">
        <span class="summary-icon green"><PhActivity :size="21" weight="duotone" /></span>
        <div><small>统计口径</small><h2>总覆盖按自然时间并集计算</h2><p>前台活动与有证据的 Provider 执行重叠时只计一次；当前总覆盖为 {{ formatDuration(store.day.value.totalDuration.value, true) }}。</p></div>
      </article>
      <article class="card event-summary">
        <span class="summary-icon violet"><PhStack :size="21" weight="duotone" /></span>
        <div><small>轨道说明</small><h2>上下对齐表示同时发生，不代表重复记录</h2><p>设备、应用、AI、语音和媒体各自保留来源；并行时段单独统计，不会覆盖原始时长。</p></div>
      </article>
    </div>

    <p class="timeline-footnote"><PhClockCounterClockwise :size="14" />当前页只呈现已有记录；采集启用前的活动不会补造。</p>
  </section>
</template>

<style scoped>
.timeline-overview { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.timeline-stat { position: relative; overflow: hidden; display: grid; gap: 5px; padding: 17px 19px; border: 1px solid var(--border-soft); border-radius: var(--radius-md); background: var(--bg-card); box-shadow: var(--shadow-card); }
.timeline-stat::before { content: ''; position: absolute; inset: 0 auto 0 0; width: 3px; background: var(--stat-color); }
.timeline-stat span { color: var(--text-secondary); font-size: 11px; }
.timeline-stat strong { font: 720 21px/1.2 var(--font-data); letter-spacing: -.35px; }
.timeline-stat small { color: var(--text-muted); font-size: 10px; }
.stat-green { --stat-color: var(--accent-green); }.stat-violet { --stat-color: var(--accent-violet); }.stat-blue { --stat-color: var(--accent-blue); }
.full-timeline { margin-top: 12px; padding: 21px; }
.track-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 14px; }
.track-header h2 { margin: 0; font-size: 16px; }
.track-header p { margin: 5px 0 0; color: var(--text-secondary); font-size: 11px; }
.track-header > span { display: inline-flex; align-items: center; gap: 5px; flex: 0 0 auto; padding: 5px 8px; border-radius: 999px; color: var(--accent-green-strong); background: var(--accent-green-soft); font-size: 10px; }
.timeline-axis { display: grid; grid-template-columns: 92px repeat(7, 1fr); gap: 12px; margin: 20px 0 8px; color: var(--text-muted); font-size: 10px; font-variant-numeric: tabular-nums; }
.timeline-axis span:not(:first-child) { text-align: center; }
.timeline-tracks { display: grid; gap: 7px; }
.timeline-legend { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 14px; margin-top: 17px; color: var(--text-secondary); font-size: 10px; }
.timeline-legend span { display: inline-flex; align-items: center; gap: 5px; }
.timeline-legend i { width: 7px; height: 7px; border-radius: 2px; }.timeline-legend .green { background: var(--accent-green); }.timeline-legend .idle { background: #d29a45; }.timeline-legend .unknown { background: #a3a8ae; }.timeline-legend .blue { background: var(--accent-blue); }.timeline-legend .violet { background: var(--accent-violet); }.timeline-legend .cyan { background: var(--accent-cyan); }.timeline-legend .orange { background: var(--accent-orange); }
.timeline-detail-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 12px; }
.event-summary { display: flex; gap: 13px; padding: 17px; }
.summary-icon { width: 38px; height: 38px; flex: 0 0 auto; display: grid; place-items: center; border-radius: 11px; }.summary-icon.green { color: var(--accent-green-strong); background: var(--accent-green-soft); }.summary-icon.violet { color: var(--accent-violet-strong); background: var(--accent-violet-soft); }
.event-summary small { color: var(--text-muted); font-size: 10px; }.event-summary h2 { margin: 4px 0 0; font-size: 12px; }.event-summary p { margin: 6px 0 0; color: var(--text-secondary); font-size: 10px; line-height: 1.6; }
.timeline-footnote { display: flex; align-items: center; justify-content: flex-end; gap: 5px; margin: 10px 2px 0; color: var(--text-muted); font-size: 10px; }
@media (max-width: 760px) { .timeline-overview,
.timeline-detail-grid { grid-template-columns: 1fr; } .timeline-axis { grid-template-columns: 72px repeat(7, 1fr); gap: 8px; } }
</style>
