<script setup lang="ts">
import { computed } from 'vue'
import type { TimeRange, TimelineSegment } from '../domain/events'
import { formatClock } from '../utils/format'
const props = defineProps<{ range: TimeRange; segments: TimelineSegment[]; label?: string }>()
const kindLabels = {
  attention: '主动注意力',
  agent: 'AI 代理',
  media: '离座播放',
  other: '其他活动',
  interaction: '前台交互',
  waiting: '静默等待',
  overlap: '并行重叠',
} as const
const positioned = computed(() => props.segments.map((segment) => ({
  ...segment,
  left: `${Math.max(0, (segment.start - props.range.start) / (props.range.end - props.range.start) * 100)}%`,
  width: `${Math.max(0.6, (segment.end - segment.start) / (props.range.end - props.range.start) * 100)}%`,
  tooltip: `${kindLabels[segment.kind ?? 'other']} · ${formatClock(segment.start)}—${formatClock(segment.end)}`,
})))
</script>

<template>
  <div class="timeline-lane">
    <span v-if="label" class="timeline-lane__label">{{ label }}</span>
    <div class="timeline-lane__track" role="list" :aria-label="label ? `${label}时间区间` : '活动时间区间'">
      <span
        v-for="(segment, index) in positioned"
        :key="index"
        class="timeline-segment"
        :class="[`is-${segment.kind ?? 'custom'}`, `is-${segment.variant ?? 'solid'}`, { muted: segment.muted }]"
        :style="{ left: segment.left, width: segment.width, '--segment-color': segment.color }"
        :aria-label="segment.tooltip"
        :data-tooltip="segment.tooltip"
        role="listitem"
        tabindex="0"
      ></span>
    </div>
  </div>
</template>

<style scoped>
.timeline-lane__track {
  overflow: visible;
  isolation: isolate;
  background:
    linear-gradient(90deg, color-mix(in srgb, var(--bg-soft) 62%, transparent), transparent),
    repeating-linear-gradient(90deg, transparent 0 calc(16.666% - 1px), var(--border-soft) calc(16.666% - 1px) 16.666%);
}

.timeline-segment {
  border: 1px solid color-mix(in srgb, currentColor 10%, transparent);
  box-shadow: 0 2px 7px color-mix(in srgb, var(--segment-color, var(--accent-blue)) 16%, transparent);
  cursor: help;
  transition: filter 150ms ease, opacity 150ms ease, transform 150ms ease;
}

.timeline-segment:hover,
.timeline-segment:focus,
.timeline-segment:focus-visible {
  z-index: 10;
  filter: saturate(1.08);
  opacity: 1;
  transform: translateY(-1px);
  outline: 2px solid color-mix(in srgb, var(--text-primary) 24%, transparent);
  outline-offset: 1px;
}

.timeline-segment::after {
  content: attr(data-tooltip);
  position: absolute;
  left: 50%;
  bottom: calc(100% + 8px);
  z-index: 20;
  width: max-content;
  max-width: 190px;
  padding: 6px 8px;
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  color: var(--text-primary);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
  font-size: 9px;
  font-weight: 600;
  line-height: 1.3;
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, 4px);
  transition: opacity 140ms ease, transform 140ms ease;
}

.timeline-segment:hover::after,
.timeline-segment:focus::after,
.timeline-segment:focus-visible::after {
  opacity: 1;
  transform: translate(-50%, 0);
}
</style>
