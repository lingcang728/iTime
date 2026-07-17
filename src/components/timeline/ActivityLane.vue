<script setup lang="ts">
import { computed } from 'vue'
import type { TimeRange, TimelineSegment } from '../../domain/events'
import { formatClock, formatDuration } from '../../utils/format'

export interface ActivitySegment extends TimelineSegment {
  title: string
}

const props = defineProps<{
  label: string
  range: TimeRange
  segments: ActivitySegment[]
}>()

const positioned = computed(() => {
  const duration = Math.max(1, props.range.end - props.range.start)
  return props.segments.flatMap((segment) => {
    const start = Math.max(props.range.start, segment.start)
    const end = Math.min(props.range.end, segment.end)
    if (end <= start) return []
    const left = (start - props.range.start) / duration * 100
    const width = Math.min(100 - left, Math.max(.7, (end - start) / duration * 100))
    return [{
      ...segment,
      start,
      end,
      left: `${left}%`,
      width: `${width}%`,
      edge: left < 14 ? 'left' : left + width > 86 ? 'right' : 'center',
      accessibleLabel: `${segment.title}，${formatClock(start)} 至 ${formatClock(end)}，${formatDuration(end - start, true)}`,
    }]
  })
})
</script>

<template>
  <div class="activity-lane">
    <span class="lane-label">{{ label }}</span>
    <div class="lane-track">
      <span v-if="!positioned.length" class="lane-empty">无记录</span>
      <span
        v-for="segment in positioned"
        :key="`${segment.start}-${segment.end}-${segment.title}`"
        class="lane-segment"
        :class="[`is-${segment.kind ?? 'other'}`, `is-${segment.variant ?? 'solid'}`, `edge-${segment.edge}`, { muted: segment.muted }]"
        :style="{ left: segment.left, width: segment.width, '--segment-color': segment.color }"
        role="img"
        tabindex="0"
        :aria-label="segment.accessibleLabel"
      >
        <span role="tooltip"><strong>{{ segment.title }}</strong>{{ formatClock(segment.start) }}–{{ formatClock(segment.end) }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.activity-lane { display: grid; grid-template-columns: 92px minmax(0, 1fr); align-items: center; gap: 12px; min-height: 40px; }
.lane-label { color: var(--text-secondary); font-size: 10px; font-weight: 600; }
.lane-track { position: relative; height: 31px; border: 1px solid var(--border-soft); border-radius: 8px; background-color: color-mix(in srgb, var(--bg-soft) 66%, var(--bg-card)); background-image: linear-gradient(to right, var(--border-soft) 1px, transparent 1px); background-size: 16.666% 100%; }
.lane-empty { position: absolute; inset: 0; display: grid; place-items: center; color: var(--text-muted); font-size: 10px; }
.lane-segment { --segment-color: var(--accent-blue); position: absolute; top: 5px; bottom: 5px; min-width: 3px; border: 1px solid color-mix(in srgb, var(--segment-color) 48%, transparent); border-radius: 5px; background: color-mix(in srgb, var(--segment-color) 82%, var(--bg-card)); box-shadow: 0 1px 2px color-mix(in srgb, var(--segment-color) 20%, transparent); cursor: help; transition: filter 150ms ease, transform 150ms var(--ease-out); }
.lane-segment.muted { opacity: .42; filter: saturate(.55); }
.lane-segment.is-outline { background: color-mix(in srgb, var(--segment-color) 16%, var(--bg-card)); }
.lane-segment.is-hatched { background: repeating-linear-gradient(135deg, color-mix(in srgb, var(--segment-color) 62%, var(--bg-card)) 0 3px, color-mix(in srgb, var(--segment-color) 18%, var(--bg-card)) 3px 6px); }
.lane-segment:hover,
.lane-segment:focus,
.lane-segment:focus-visible { z-index: 3; opacity: 1; filter: saturate(1.08); transform: translateY(-2px); }
.lane-segment > span { position: absolute; left: 50%; bottom: calc(100% + 8px); display: grid; gap: 2px; min-width: 108px; padding: 7px 9px; transform: translateX(-50%); border: 1px solid var(--border-strong); border-radius: 8px; color: var(--text-secondary); background: var(--bg-elevated); box-shadow: var(--shadow-card); font-size: 10px; font-variant-numeric: tabular-nums; white-space: nowrap; opacity: 0; pointer-events: none; }
.lane-segment > span strong { color: var(--text-primary); font-size: 10px; }
.lane-segment.edge-left > span { left: 0; transform: none; }
.lane-segment.edge-right > span { right: 0; left: auto; transform: none; }
.lane-segment:hover > span,
.lane-segment:focus > span,
.lane-segment:focus-visible > span { opacity: 1; }
@media (max-width: 720px) { .activity-lane { grid-template-columns: 72px minmax(0, 1fr); gap: 8px; } }
</style>
