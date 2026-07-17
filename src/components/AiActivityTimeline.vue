<script setup lang="ts">
import { computed } from 'vue'
import type { AiToolSummary, ForegroundAppInterval, TimeRange } from '../domain/events'
import { intersectRanges } from '../domain/intervals'
import { formatClock } from '../utils/format'

type SegmentKind = 'human' | 'work' | 'interaction' | 'wait' | 'overlap'

interface SegmentView extends TimeRange {
  id: string
  kind: SegmentKind
  label: string
  left: string
  width: string
}

interface LaneView {
  id: string
  label: string
  meta: string
  segments: SegmentView[]
}

const props = defineProps<{
  range: TimeRange
  foreground: ForegroundAppInterval[]
  tools: AiToolSummary[]
}>()

const ticks = [0, 4, 8, 12, 16, 20, 24]

function segment(id: string, kind: SegmentKind, interval: TimeRange, label: string): SegmentView {
  const duration = Math.max(1, props.range.end - props.range.start)
  const left = Math.max(0, Math.min(100, (interval.start - props.range.start) / duration * 100))
  const right = Math.max(left, Math.min(100, (interval.end - props.range.start) / duration * 100))
  return { ...interval, id, kind, label, left: `${left}%`, width: `${Math.min(100 - left, Math.max(.45, right - left))}%` }
}

function timeLabel(interval: TimeRange): string {
  return `${formatClock(interval.start)}–${formatClock(interval.end)}`
}

const lanes = computed<LaneView[]>(() => {
  const human: LaneView = {
    id: 'human',
    label: '人的前台活动',
    meta: `${props.foreground.length} 个区间`,
    segments: props.foreground.map((item) => segment(item.id, 'human', item, `${item.appName} · ${timeLabel(item)}`)),
  }
  const tools = props.tools.map((tool): LaneView => {
    const work = tool.workIntervals.map((item) => segment(`work:${item.id}`, 'work', item, `Provider 执行 · ${timeLabel(item)}`))
    const interactions = tool.interactionIntervals.map((item) => segment(`interaction:${item.id}`, 'interaction', item, `AI 前台活跃 · ${timeLabel(item)}`))
    const waits = tool.waitIntervals.map((item, index) => segment(`${tool.toolId}-wait-${index}`, 'wait', item, `静默等待 · ${timeLabel(item)}`))
    const overlap = intersectRanges(tool.workIntervals, props.foreground).map((item, index) => segment(`${tool.toolId}-overlap-${index}`, 'overlap', item, `并行重叠 · ${timeLabel(item)}`))
    return {
      id: tool.toolId,
      label: tool.toolName,
      meta: work.length ? `${work.length} 个执行区间` : `${interactions.length} 个前台区间`,
      segments: [...work, ...waits, ...interactions, ...overlap],
    }
  })
  return [human, ...tools]
})
</script>

<template>
  <div class="timeline" aria-label="AI 工具活动时间线">
    <div class="timeline__axis" aria-hidden="true">
      <span></span>
      <div><i v-for="tick in ticks" :key="tick" :style="{ left: `${tick / 24 * 100}%` }">{{ String(tick).padStart(2, '0') }}:00</i></div>
    </div>
    <div v-if="lanes.length === 1" class="timeline__empty">当天尚未采集到 AI 工具活动。</div>
    <div v-for="lane in lanes" :key="lane.id" class="timeline__lane">
      <div class="timeline__name"><strong>{{ lane.label }}</strong><span>{{ lane.meta }}</span></div>
      <div class="timeline__track">
        <button
          v-for="item in lane.segments"
          :key="item.id"
          type="button"
          class="timeline__segment"
          :class="`is-${item.kind}`"
          :style="{ left: item.left, width: item.width }"
          :aria-label="item.label"
        >
          <span role="tooltip">{{ item.label }}</span>
        </button>
      </div>
    </div>
    <div class="timeline__legend" aria-label="图例">
      <span class="is-human">人的前台活动</span><span class="is-work">Provider 执行</span>
      <span class="is-interaction">AI 前台活跃</span><span class="is-wait">静默等待</span><span class="is-overlap">并行重叠</span>
    </div>
  </div>
</template>

<style scoped>
.timeline { margin-top: 8px; }
.timeline__axis, .timeline__lane { display: grid; grid-template-columns: 112px minmax(0, 1fr); gap: 12px; }
.timeline__axis { align-items: end; height: 24px; color: var(--text-muted); font-size: 10px; }
.timeline__axis > div { height: 18px; position: relative; }
.timeline__axis i { position: absolute; bottom: 0; font-style: normal; transform: translateX(-50%); }
.timeline__axis i:first-child { transform: none; }
.timeline__axis i:last-child { transform: translateX(-100%); }

.timeline__lane { align-items: center; min-height: 40px; border-top: 1px solid color-mix(in srgb, var(--border-soft) 70%, transparent); }
.timeline__name { min-width: 0; display: grid; gap: 2px; }
.timeline__name strong { overflow: hidden; color: var(--text-primary); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.timeline__name span { color: var(--text-muted); font-size: 10px; }
.timeline__track {
  height: 27px;
  position: relative;
  border-radius: 7px;
  background-image: linear-gradient(90deg, color-mix(in srgb, var(--border-soft) 65%, transparent) 1px, transparent 1px);
  background-size: calc(100% / 6) 100%;
}
.timeline__track::after { content: ''; position: absolute; inset: 13px 0 auto; border-top: 1px solid var(--border-soft); }

.timeline__segment { min-width: 3px; position: absolute; z-index: 2; top: 8px; height: 11px; padding: 0; border: 0; border-radius: 4px; background: var(--accent-violet); cursor: crosshair; }
.timeline__segment.is-human { top: 10px; height: 7px; background: color-mix(in srgb, var(--accent-green) 74%, white); }
.timeline__segment.is-interaction { z-index: 4; top: 5px; height: 17px; border: 1px solid var(--accent-blue); background: transparent; }
.timeline__segment.is-wait { top: 11px; height: 5px; border: 1px dashed color-mix(in srgb, var(--accent-violet) 68%, var(--border-soft)); background: var(--bg-card); }
.timeline__segment.is-overlap { z-index: 5; top: 9px; height: 9px; background: repeating-linear-gradient(90deg, var(--accent-violet) 0 3px, var(--accent-green) 3px 6px); }
.timeline__segment:focus-visible { outline: 2px solid var(--text-primary); outline-offset: 2px; }
.timeline__segment [role="tooltip"] {
  width: max-content;
  max-width: 190px;
  position: absolute;
  z-index: 10;
  left: 50%;
  bottom: calc(100% + 7px);
  padding: 6px 8px;
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  color: var(--text-primary);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
  font-size: 10px;
  line-height: 1.4;
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, 3px);
  transition: opacity 120ms ease, transform 120ms ease;
}
.timeline__segment:hover [role="tooltip"], .timeline__segment:focus [role="tooltip"] { opacity: 1; transform: translate(-50%, 0); }

.timeline__legend { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 12px; margin-top: 9px; color: var(--text-muted); font-size: 10px; }
.timeline__legend span::before { content: ''; width: 9px; height: 5px; display: inline-block; margin-right: 5px; border-radius: 2px; background: var(--accent-violet); vertical-align: 1px; }
.timeline__legend .is-human::before { background: var(--accent-green); }
.timeline__legend .is-interaction::before { border: 1px solid var(--accent-blue); background: transparent; }
.timeline__legend .is-wait::before { border: 1px dashed var(--accent-violet); background: transparent; }
.timeline__legend .is-overlap::before { background: linear-gradient(90deg, var(--accent-violet) 50%, var(--accent-green) 50%); }
.timeline__empty { margin: 18px 0 8px 124px; color: var(--text-muted); font-size: 10px; }

@media (max-width: 840px) {
  .timeline__axis, .timeline__lane { grid-template-columns: 88px minmax(0, 1fr); gap: 8px; }
  .timeline__legend { justify-content: flex-start; padding-left: 96px; }
}
</style>
