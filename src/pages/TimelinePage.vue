<script setup lang="ts">
import { computed } from 'vue'
import { PhActivity, PhCaretDown, PhCaretRight, PhWaveform } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import TimelineLane from '../components/TimelineLane.vue'
import type { AiWorkInterval, DeviceStateInterval, ForegroundAppInterval, InputActivityMinuteBucket, MediaPlaybackInterval, TimeEvent, VoiceInputInterval } from '../domain/events'
import { useAppStore } from '../stores/appStore'
import { formatDuration } from '../utils/format'

const store = useAppStore()
const colors = { active: '#67b783', idle: '#d9b45d', locked: '#9aa3b2', sleep: '#c8ccd4', unknown: '#d7dbe2' }
const byType = <T extends TimeEvent>(type: T['type']) => store.day.value.events.filter((event): event is T => event.type === type)
const deviceSegments = computed(() => byType<DeviceStateInterval>('device').map((event) => ({ start: event.start, end: event.end, color: colors[event.state], muted: event.state !== 'active' })))
const appSegments = computed(() => byType<ForegroundAppInterval>('foreground').map((event) => ({ start: event.start, end: event.end, color: event.color })))
const aiSegments = computed(() => byType<AiWorkInterval>('aiWork').map((event) => ({ start: event.start, end: event.end, color: event.accuracyLabel === 'precise' ? '#806be1' : '#b0a5ea' })))
const voiceSegments = computed(() => byType<VoiceInputInterval>('voice').map((event) => ({ start: event.start, end: event.end, color: '#55b8b3' })))
const mediaSegments = computed(() => byType<MediaPlaybackInterval>('media').map((event) => ({ start: event.start, end: event.end, color: '#e08b5f' })))
const inputBuckets = computed(() => byType<InputActivityMinuteBucket>('input'))
const maxInput = computed(() => Math.max(1, ...inputBuckets.value.map((bucket) => bucket.keyStrokes + bucket.leftClicks * 4)))
const densityBars = computed(() => inputBuckets.value.map((bucket) => ({
  left: `${(bucket.start - store.day.value.range.start) / (store.day.value.range.end - store.day.value.range.start) * 100}%`,
  height: `${Math.max(4, (bucket.keyStrokes + bucket.leftClicks * 4) / maxInput.value * 100)}%`,
})))
</script>

<template>
  <section class="page timeline-page">
    <PageHeader title="今日时间线" subtitle="从同一份事件记录核对设备、应用、AI 与输入活动" />
    <div class="timeline-overview">
      <article class="timeline-stat"><span>前台活动</span><strong>{{ formatDuration(store.day.value.foregroundActivity.value, true) }}</strong><small>设备活跃与前台应用交集</small></article>
      <article class="timeline-stat"><span>AI 覆盖</span><strong>{{ formatDuration(store.day.value.aiCoverage.value, true) }}</strong><small>至少一个代理有效工作</small></article>
      <article class="timeline-stat"><span>并行重叠</span><strong>{{ formatDuration(store.day.value.parallelOverlap.value, true) }}</strong><small>人的活动与 AI 覆盖交集</small></article>
    </div>
    <article class="card full-timeline" tabindex="0" aria-label="六轨今日时间线">
      <div class="section-heading">
        <div><h2>活动轨道</h2><p>所有区间采用本地时区与半开区间 [start, end)</p></div>
        <button class="density-toggle" type="button" :aria-pressed="store.state.showInputDensity" @click="store.state.showInputDensity = !store.state.showInputDensity">
          <PhCaretDown v-if="store.state.showInputDensity" :size="15" /><PhCaretRight v-else :size="15" />输入密度
        </button>
      </div>
      <div class="timeline-axis timeline-axis--labeled"><span></span><span>00:00</span><span>04:00</span><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span><span>24:00</span></div>
      <div class="timeline-tracks">
        <TimelineLane label="设备状态" :range="store.day.value.range" :segments="deviceSegments" />
        <TimelineLane label="前台应用" :range="store.day.value.range" :segments="appSegments" />
        <TimelineLane label="AI 代理" :range="store.day.value.range" :segments="aiSegments" />
        <TimelineLane label="语音输入" :range="store.day.value.range" :segments="voiceSegments" />
        <TimelineLane label="媒体播放" :range="store.day.value.range" :segments="mediaSegments" />
        <div v-if="store.state.showInputDensity" class="density-lane">
          <span class="timeline-lane__label">输入密度</span>
          <div class="density-track"><i v-for="(bar, index) in densityBars" :key="index" :style="bar"></i></div>
        </div>
      </div>
      <div class="timeline-legend">
        <span><i class="dot green"></i>活跃</span><span><i class="dot blue"></i>应用</span><span><i class="dot violet"></i>AI 代理</span><span><i class="dot cyan"></i>语音</span><span><i class="dot orange"></i>媒体播放</span>
      </div>
    </article>
    <div class="timeline-detail-grid">
      <article class="card event-summary"><PhActivity :size="21" weight="duotone" /><div><span>统计口径</span><strong>总时长不会重复加入覆盖或重叠</strong><p>前台活动 {{ formatDuration(store.day.value.foregroundActivity.value, true) }} + AI 有效代理工时 {{ formatDuration(store.day.value.aiEffective.value, true) }} = {{ formatDuration(store.day.value.totalDuration.value, true) }}</p></div></article>
      <article class="card event-summary"><PhWaveform :size="21" weight="duotone" /><div><span>轨道说明</span><strong>媒体播放不是键鼠输入的替代项</strong><p>输入密度独立折叠，避免遮挡关键活动区间。</p></div></article>
    </div>
  </section>
</template>
