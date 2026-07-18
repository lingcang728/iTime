<script setup lang="ts">
import { computed } from 'vue'
import type { TimeRange, TimelineSegment } from '../domain/events'
import TimelineLane from './TimelineLane.vue'
import { buildHomeTimelineBands } from './homeTimelineBands'

const props = defineProps<{ range: TimeRange; segments: TimelineSegment[] }>()
const bands = computed(() => buildHomeTimelineBands(props.range, props.segments))
const axisTicks = ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00']
</script>

<template>
  <div class="home-timeline-bands" aria-label="今日六段时间线">
    <div v-for="band in bands" :key="band.label" class="home-timeline-band">
      <span>{{ band.label }}</span>
      <TimelineLane :range="band.range" :segments="band.segments" />
    </div>
    <div class="home-timeline-axis time-axis" aria-hidden="true">
      <span v-for="tick in axisTicks" :key="tick">{{ tick }}</span>
    </div>
  </div>
</template>

<style scoped>
.home-timeline-bands {
  min-height: 0;
  display: grid;
  align-content: stretch;
  grid-template-rows: repeat(6, minmax(27px, 1fr)) 22px;
  margin-top: 10px;
}

.home-timeline-band {
  min-width: 0;
  display: grid;
  grid-template-columns: 126px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  border-bottom: 1px solid var(--border-soft);
}

.home-timeline-band > span {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.home-timeline-band :deep(.timeline-lane) {
  margin: 0;
}

.home-timeline-band :deep(.timeline-lane__track) {
  height: 22px;
  overflow: visible;
  border: 0;
  border-radius: 0;
  background: linear-gradient(var(--border-soft), var(--border-soft)) center / 100% 2px no-repeat;
}

.home-timeline-band :deep(.timeline-segment) {
  top: 7px;
  bottom: 7px;
  min-width: 5px;
  border: 0;
  border-radius: 3px;
  box-shadow: none;
}

.home-timeline-axis {
  display: flex;
  justify-content: space-between;
  align-items: end;
  margin-left: 138px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
}

.home-timeline-axis span {
  transform: none;
}

.home-timeline-axis span:first-child {
  transform: none;
}

.home-timeline-axis span:last-child {
  text-align: right;
}

@media (max-width: 1050px) {
  .home-timeline-band {
    grid-template-columns: 104px minmax(0, 1fr);
  }

  .home-timeline-axis {
    margin-left: 116px;
  }
}
</style>
