<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PhCheckCircle,
  PhClockCounterClockwise,
  PhDesktop,
  PhInfo,
  PhIntersect,
  PhRobot,
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
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration } from '../utils/format'

const store = useAppStore()
const notesOpen = ref(false)
const deviceStyles = {
  active: { color: 'var(--accent-green)', kind: 'attention', variant: 'solid', muted: false },
  idle: { color: 'var(--text-muted)', kind: 'waiting', variant: 'hatched', muted: true },
  locked: { color: 'var(--text-muted)', kind: 'waiting', variant: 'outline', muted: true },
  sleep: { color: 'var(--text-muted)', kind: 'waiting', variant: 'outline', muted: true },
  unknown: { color: 'var(--text-muted)', kind: 'waiting', variant: 'hatched', muted: true },
} as const
const deviceNames = { active: '设备活跃', idle: '暂时空闲', locked: '设备锁定', sleep: '设备休眠', unknown: '状态未知' }
const byType = <T extends TimeEvent>(type: T['type']) => store.day.value.events.filter((event): event is T => event.type === type)

const deviceSegments = computed<ActivitySegment[]>(() => byType<DeviceStateInterval>('device').map((event) => ({
  start: event.start,
  end: event.end,
  ...deviceStyles[event.state],
  title: deviceNames[event.state],
})))
const appSegments = computed<ActivitySegment[]>(() => byType<ForegroundAppInterval>('foreground').map((event) => ({
  start: event.start,
  end: event.end,
  color: 'var(--accent-green)',
  kind: 'other',
  title: event.appName,
})))
const aiSegments = computed<ActivitySegment[]>(() => byType<AiInteractionInterval>('aiInteraction').map((event) => ({
  start: event.start,
  end: event.end,
  color: 'var(--accent-green)',
  kind: 'interaction',
  variant: 'outline',
  title: `${event.toolName}（前台活跃）`,
})))
const mediaSegments = computed<ActivitySegment[]>(() => byType<MediaPlaybackInterval>('media').map((event) => ({
  start: event.start,
  end: event.end,
  color: 'var(--text-muted)',
  kind: 'media',
  variant: event.awayPlayback ? 'outline' : 'solid',
  muted: event.awayPlayback,
  title: `${event.appName}${event.awayPlayback ? '（离开播放）' : ''}`,
})))
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const pageSubtitle = computed(() => ({
  ready: '用同一日活动记录核对设备、前台应用与 AI 前台活跃',
  degraded: '本机活动记录部分可用；页面按现有证据呈现',
  preview: '展示设备、前台应用与 AI 前台活跃在一天中的关系',
  loading: '正在读取本机活动记录',
  unavailable: '本机活动记录暂时不可用',
}[store.state.activityDataStatus]))
const sourceLabel = computed(() => ({
  ready: '本机活动记录',
  degraded: '部分本机记录',
  preview: '预览数据',
  loading: '正在读取',
  unavailable: '暂不可用',
}[store.state.activityDataStatus]))
const unavailableTitle = computed(() => store.state.activityDataStatus === 'loading' ? '正在读取活动记录' : '活动记录暂不可用')
</script>

<template>
  <section class="page timeline-page">
    <PageHeader title="今日时间线" :subtitle="pageSubtitle" />

    <div class="timeline-overview">
      <MetricCard
        label="前台活动"
        :value="formatDuration(store.day.value.foregroundActivity.value, true)"
        detail="设备活跃且有前台应用"
        :icon="PhDesktop"
        tone="accent"
        info="由设备活跃区间与前台应用区间取交集得到。"
      />
      <MetricCard
        label="AI 前台活跃"
        :value="formatDuration(store.day.value.aiInteraction.value, true)"
        detail="设备活跃且 AI 工具处于前台"
        :icon="PhRobot"
        tone="neutral"
        info="只记录可观察的前台身份与活跃状态，不读取对话或输入内容。"
      />
      <MetricCard
        label="Provider 并行"
        :value="formatDuration(store.day.value.parallelOverlap.value, true)"
        detail="仅有执行事件时计算"
        :icon="PhIntersect"
        tone="neutral"
        info="前台活动与 Provider 执行证据在自然时间上的交集。"
      />
    </div>

    <article class="full-timeline" aria-labelledby="activity-tracks-title">
      <header class="track-header">
        <div><h2 id="activity-tracks-title">一天的活动分布</h2><p>四条轨道共用同一时间刻度；悬停或聚焦色块查看区间</p></div>
        <div class="track-actions">
          <span><PhCheckCircle :size="15" />{{ sourceLabel }}</span>
          <button type="button" class="timeline-info-button" aria-label="查看统计口径与轨道说明" aria-controls="timeline-notes" :aria-expanded="notesOpen" @click="notesOpen = !notesOpen" @keydown.escape="notesOpen = false"><PhInfo :size="16" /></button>
          <Transition name="popover">
            <aside v-if="notesOpen" id="timeline-notes" class="timeline-popover" role="dialog" aria-label="统计口径与轨道说明">
              <button type="button" aria-label="关闭说明" @click="notesOpen = false"><PhX :size="14" /></button>
              <div><span>统计口径</span><strong>总覆盖按自然时间并集计算</strong><p>前台活动与有证据的 Provider 执行重叠时只计一次；当前总覆盖为 {{ formatDuration(store.day.value.totalDuration.value, true) }}。</p></div>
              <div><span>轨道说明</span><strong>上下对齐表示同时发生</strong><p>设备、应用、AI 与媒体各自保留来源；并行时段单独统计，不代表重复记录。</p></div>
            </aside>
          </Transition>
        </div>
      </header>
      <template v-if="activityDataAvailable">
        <div class="timeline-axis"><span></span><span>00:00</span><span>04:00</span><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span><span>24:00</span></div>
        <div class="timeline-tracks">
          <ActivityLane label="设备状态" :range="store.day.value.range" :segments="deviceSegments" />
          <ActivityLane label="前台应用" :range="store.day.value.range" :segments="appSegments" />
          <ActivityLane label="AI 前台" :range="store.day.value.range" :segments="aiSegments" />
          <ActivityLane label="媒体播放" :range="store.day.value.range" :segments="mediaSegments" />
        </div>
        <div class="timeline-legend" aria-label="时间线颜色说明">
          <span><i class="sage" />设备活跃</span><span><i class="muted-hatch" />设备非活跃</span><span><i class="sage-soft" />前台应用</span><span><i class="sage-outline" />AI 前台</span><span><i class="neutral" />媒体播放</span>
        </div>
      </template>
      <div v-else class="section-state timeline-source-state" :data-state="store.state.activityDataStatus">
        <strong>{{ unavailableTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
      </div>
    </article>

    <p class="timeline-footnote"><PhClockCounterClockwise :size="14" />当前页只呈现已有记录；采集启用前的活动不会补造。</p>
  </section>
</template>
