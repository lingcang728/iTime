<script setup lang="ts">
import { computed } from 'vue'
import type { TimeRange } from '../domain/events'
const props = defineProps<{ range: TimeRange; segments: Array<TimeRange & { color?: string; muted?: boolean }>; label?: string }>()
const positioned = computed(() => props.segments.map((segment) => ({
  ...segment,
  left: `${Math.max(0, (segment.start - props.range.start) / (props.range.end - props.range.start) * 100)}%`,
  width: `${Math.max(0.6, (segment.end - segment.start) / (props.range.end - props.range.start) * 100)}%`,
})))
</script>

<template>
  <div class="timeline-lane">
    <span v-if="label" class="timeline-lane__label">{{ label }}</span>
    <div class="timeline-lane__track">
      <span v-for="(segment, index) in positioned" :key="index" class="timeline-segment" :class="{ muted: segment.muted }" :style="{ left: segment.left, width: segment.width, background: segment.color }"></span>
    </div>
  </div>
</template>

