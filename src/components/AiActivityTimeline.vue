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
  edge: 'left' | 'center' | 'right'
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

function segment(id: string, kind: SegmentKind, interval: TimeRange, label: string): SegmentView[] {
  const duration = Math.max(1, props.range.end - props.range.start)
  const start = Math.max(props.range.start, interval.start)
  const end = Math.min(props.range.end, interval.end)
  if (end <= start) return []
  const left = (start - props.range.start) / duration * 100
  const width = Math.min(100 - left, Math.max(.45, (end - start) / duration * 100))
  return [{
    start,
    end,
    id,
    kind,
    label: `${label} · ${timeLabel({ start, end })}`,
    left: `${left}%`,
    width: `${width}%`,
    edge: left < 14 ? 'left' : left + width > 86 ? 'right' : 'center',
  }]
}

function timeLabel(interval: TimeRange): string {
  return `${formatClock(interval.start)}–${formatClock(interval.end)}`
}

const lanes = computed<LaneView[]>(() => {
  const human: LaneView = {
    id: 'human',
    label: '人的前台活动',
    meta: `${props.foreground.length} 个区间`,
    segments: props.foreground.flatMap((item) => segment(item.id, 'human', item, item.appName)),
  }
  const tools = props.tools.map((tool): LaneView => {
    const work = tool.workIntervals.flatMap((item) => segment(`work:${item.id}`, 'work', item, 'Provider 执行'))
    const interactions = tool.interactionIntervals.flatMap((item) => segment(`interaction:${item.id}`, 'interaction', item, 'AI 前台活跃'))
    const waits = tool.waitIntervals.flatMap((item, index) => segment(`${tool.toolId}-wait-${index}`, 'wait', item, '静默等待'))
    const overlap = intersectRanges(tool.workIntervals, props.foreground).flatMap((item, index) => segment(`${tool.toolId}-overlap-${index}`, 'overlap', item, '并行重叠'))
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
          :class="[`is-${item.kind}`, `edge-${item.edge}`]"
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
.timeline {
  margin-top: var(--space-3);
}

.timeline__axis,
.timeline__lane {
  display: grid;
  grid-template-columns: 112px minmax(0, 1fr);
  gap: var(--space-3);
}

.timeline__axis {
  align-items: end;
  height: 24px;
  color: var(--text-muted);
  font-family: var(--font-data);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
}

.timeline__axis > div {
  height: 18px;
  position: relative;
}

.timeline__axis i {
  position: absolute;
  bottom: 0;
  font-style: normal;
  transform: translateX(-50%);
}

.timeline__axis i:first-child { transform: none; }
.timeline__axis i:last-child { transform: translateX(-100%); }

.timeline__lane {
  align-items: center;
  min-height: 42px;
  border-top: 1px solid color-mix(in srgb, var(--border-soft) 78%, transparent);
}

.timeline__lane:last-of-type {
  border-bottom: 1px solid color-mix(in srgb, var(--border-soft) 78%, transparent);
}

.timeline__name {
  min-width: 0;
  display: grid;
  gap: 2px;
}

.timeline__name strong {
  overflow: hidden;
  color: var(--text-primary);
  font-size: var(--text-xs);
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.timeline__name span {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.timeline__track {
  height: 28px;
  position: relative;
  background-image: linear-gradient(90deg, color-mix(in srgb, var(--border-soft) 72%, transparent) 1px, transparent 1px);
  background-size: calc(100% / 6) 100%;
}

.timeline__track::after {
  content: '';
  position: absolute;
  inset: 14px 0 auto;
  border-top: 1px solid var(--border-soft);
}

.timeline__segment {
  min-width: 3px;
  position: absolute;
  z-index: 2;
  top: 8px;
  height: 12px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 4px;
  background: var(--accent-green);
  cursor: help;
  transition: opacity 150ms ease, transform 150ms var(--ease-out), border-color 150ms ease;
}

.timeline__segment.is-human {
  top: 10px;
  height: 8px;
  background: color-mix(in srgb, var(--text-muted) 62%, transparent);
}

.timeline__segment.is-interaction {
  z-index: 4;
  top: 5px;
  height: 18px;
  border-color: var(--accent-green);
  background: color-mix(in srgb, var(--accent-green-soft) 42%, transparent);
}

.timeline__segment.is-wait {
  top: 10px;
  height: 8px;
  border: 1px dashed color-mix(in srgb, var(--text-muted) 74%, var(--border-soft));
  background: transparent;
}

.timeline__segment.is-overlap {
  z-index: 5;
  top: 8px;
  height: 12px;
  border-color: color-mix(in srgb, var(--accent-green) 72%, var(--border-soft));
  background: repeating-linear-gradient(135deg, var(--accent-green) 0 3px, color-mix(in srgb, var(--accent-green-soft) 74%, var(--bg-card)) 3px 6px);
}

.timeline__segment:hover,
.timeline__segment:focus-visible {
  z-index: 6;
  opacity: .88;
  transform: translateY(-1px);
}

.timeline__segment:focus-visible {
  outline: 2px solid var(--text-primary);
  outline-offset: 2px;
}

.timeline__segment [role="tooltip"] {
  width: max-content;
  max-width: 210px;
  position: absolute;
  z-index: 10;
  left: 50%;
  bottom: calc(100% + 7px);
  padding: 6px 8px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-popover);
  font-size: var(--text-xs);
  font-weight: 550;
  line-height: 1.4;
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, 3px);
  transition: opacity 140ms ease, transform 140ms ease;
}

.timeline__segment.edge-left [role="tooltip"] {
  left: 0;
  transform: translate(0, 3px);
}

.timeline__segment.edge-right [role="tooltip"] {
  right: 0;
  left: auto;
  transform: translate(0, 3px);
}

.timeline__segment:hover [role="tooltip"],
.timeline__segment:focus [role="tooltip"] {
  opacity: 1;
  transform: translate(-50%, 0);
}

.timeline__segment.edge-left:hover [role="tooltip"],
.timeline__segment.edge-left:focus [role="tooltip"],
.timeline__segment.edge-right:hover [role="tooltip"],
.timeline__segment.edge-right:focus [role="tooltip"] {
  transform: translate(0, 0);
}

.timeline__legend {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--space-3);
  margin-top: var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.timeline__legend span::before {
  content: '';
  width: 11px;
  height: 6px;
  display: inline-block;
  margin-right: 5px;
  border-radius: 2px;
  background: var(--accent-green);
  vertical-align: 1px;
}

.timeline__legend .is-human::before {
  background: color-mix(in srgb, var(--text-muted) 62%, transparent);
}

.timeline__legend .is-interaction::before {
  border: 1px solid var(--accent-green);
  background: transparent;
}

.timeline__legend .is-wait::before {
  border: 1px dashed var(--text-muted);
  background: transparent;
}

.timeline__legend .is-overlap::before {
  border: 1px solid color-mix(in srgb, var(--accent-green) 72%, var(--border-soft));
  background: repeating-linear-gradient(135deg, var(--accent-green) 0 3px, var(--accent-green-soft) 3px 6px);
}

.timeline__empty {
  margin: var(--space-4) 0 var(--space-2) 124px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

@media (max-width: 840px) {
  .timeline__axis,
  .timeline__lane {
    grid-template-columns: 88px minmax(0, 1fr);
    gap: var(--space-2);
  }

  .timeline__legend {
    justify-content: flex-start;
    padding-left: 96px;
  }

  .timeline__empty {
    margin-left: 96px;
  }
}
</style>
