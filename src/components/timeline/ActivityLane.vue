<script setup lang="ts">
import { computed, type Component } from 'vue'
import type { TimeRange, TimelineSegment } from '../../domain/events'
import { formatClock, formatDuration } from '../../utils/format'

export interface ActivitySegment extends TimelineSegment {
  title: string
}

const props = defineProps<{
  label: string
  range: TimeRange
  segments: ActivitySegment[]
  icon?: Component
}>()

const positioned = computed(() => {
  const duration = Math.max(1, props.range.end - props.range.start)
  return props.segments.flatMap((segment) => {
    const start = Math.max(props.range.start, segment.start)
    const end = Math.min(props.range.end, segment.end)
    if (end <= start) return []
    const left = (start - props.range.start) / duration * 100
    const width = Math.min(100 - left, Math.max(.12, (end - start) / duration * 100))
    return [{
      ...segment,
      start,
      end,
      left: `${left}%`,
      width: `${width}%`,
      widthPercent: width,
      edge: left < 14 ? 'left' : left + width > 86 ? 'right' : 'center',
      accessibleLabel: `${segment.title}，${formatClock(start)} 至 ${formatClock(end)}，${formatDuration(end - start, true)}`,
    }]
  })
})
</script>

<template>
  <div class="activity-lane">
    <span class="lane-label"><component :is="icon" v-if="icon" :size="20" weight="regular" aria-hidden="true" />{{ label }}</span>
    <div class="lane-track" role="list" :aria-label="`${label}时间区间`">
      <span v-if="!positioned.length" class="lane-empty">无记录</span>
      <span
        v-for="segment in positioned"
        :key="`${segment.start}-${segment.end}-${segment.title}`"
        class="lane-segment"
        :class="[`is-${segment.kind ?? 'other'}`, `is-${segment.variant ?? 'solid'}`, `edge-${segment.edge}`, { muted: segment.muted }]"
        :style="{ left: segment.left, width: segment.width, '--segment-color': segment.color }"
        role="listitem"
        tabindex="0"
        :aria-label="segment.accessibleLabel"
      >
        <b v-if="segment.widthPercent >= 8 && !segment.muted && segment.variant !== 'hatched'" class="lane-segment__label" aria-hidden="true">{{ segment.title }}</b>
        <span role="tooltip"><strong>{{ segment.title }}</strong>{{ formatClock(segment.start) }}–{{ formatClock(segment.end) }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.activity-lane {
  display: grid;
  grid-template-columns: 170px minmax(0, 1fr);
  align-items: center;
  gap: var(--space-3);
  min-height: 68px;
  padding: 0 18px;
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-card) 72%, var(--bg-soft));
}

.lane-label {
  display: flex;
  align-items: center;
  gap: 13px;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 650;
}

.lane-label svg { color: var(--text-secondary); }

.lane-track {
  position: relative;
  height: 36px;
  background-image:
    linear-gradient(to right, color-mix(in srgb, var(--border-strong) 72%, transparent) 1px, transparent 1px),
    linear-gradient(to right, color-mix(in srgb, var(--border-soft) 48%, transparent) 1px, transparent 1px);
  background-size: 11.111% 100%, 2.777% 100%;
}

.lane-track::after {
  content: '';
  position: absolute;
  inset: 18px 0 auto;
  border-top: 1px solid var(--border-soft);
}

.lane-empty {
  position: absolute;
  z-index: 1;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.lane-segment {
  --segment-color: var(--accent-green);
  position: absolute;
  z-index: 2;
  top: 8px;
  height: 20px;
  min-width: 2px;
  border: 1px solid color-mix(in srgb, var(--segment-color) 44%, transparent);
  border-radius: 3px;
  background: color-mix(in srgb, var(--segment-color) 78%, var(--bg-card));
  cursor: help;
  transition: opacity 150ms ease, transform 150ms var(--ease-out), border-color 150ms ease;
}

.lane-segment.is-attention {
  background: var(--accent-strong);
}

.lane-segment.is-other {
  border-color: var(--accent-strong);
  background: var(--accent-strong);
}

.lane-segment.is-interaction {
  top: 8px;
  height: 20px;
  border-color: var(--accent-green);
  background: color-mix(in srgb, var(--accent-green-soft) 38%, transparent);
}

.lane-segment.is-media {
  border-color: color-mix(in srgb, var(--text-muted) 55%, var(--border-soft));
  background: color-mix(in srgb, var(--text-muted) 62%, transparent);
}

.lane-segment.is-waiting {
  border-color: color-mix(in srgb, var(--text-muted) 58%, var(--border-soft));
}

.lane-segment.muted {
  opacity: .56;
}

.lane-segment.is-outline {
  background: transparent;
}

.lane-segment.is-hatched {
  background: repeating-linear-gradient(135deg, color-mix(in srgb, var(--segment-color) 48%, transparent) 0 3px, transparent 3px 6px);
}

.lane-segment__label {
  position: absolute;
  inset: 0 5px;
  overflow: hidden;
  color: var(--text-inverse);
  font-size: 10px;
  font-weight: 500;
  line-height: 18px;
  text-align: center;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lane-segment.is-outline .lane-segment__label { color: var(--text-secondary); }

.lane-segment:hover,
.lane-segment:focus,
.lane-segment:focus-visible {
  z-index: 4;
  opacity: 1;
  transform: translateY(-1px);
}

.lane-segment:focus-visible {
  outline: 2px solid var(--text-primary);
  outline-offset: 2px;
}

.lane-segment > span {
  position: absolute;
  left: 50%;
  bottom: calc(100% + 8px);
  display: grid;
  gap: 2px;
  min-width: 116px;
  padding: 7px 9px;
  transform: translateX(-50%);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-popover);
  font-family: var(--font-data);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 140ms ease;
}

.lane-segment > span strong {
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: var(--text-xs);
}

.lane-segment.edge-left > span {
  left: 0;
  transform: none;
}

.lane-segment.edge-right > span {
  right: 0;
  left: auto;
  transform: none;
}

.lane-segment:hover > span,
.lane-segment:focus > span,
.lane-segment:focus-visible > span {
  opacity: 1;
}

@media (max-width: 720px) {
  .activity-lane {
    grid-template-columns: 112px minmax(0, 1fr);
    gap: var(--space-2);
  }
}
</style>
