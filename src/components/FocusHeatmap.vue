<script setup lang="ts">
import { computed, onBeforeUpdate, ref } from 'vue'
import type { HeatmapDay } from '../data/focusHeatmap'

const props = defineProps<{ days: HeatmapDay[] }>()
const weekdayLabels = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
const hovered = ref<HeatmapDay | null>(null)
const focused = ref<HeatmapDay | null>(null)
const locked = ref<HeatmapDay | null>(null)
const cellRefs: HTMLButtonElement[] = []
const active = computed(() => locked.value ?? focused.value ?? hovered.value)
const weekLabels = computed(() => Array.from({ length: 7 }, (_, weekIndex) => {
  const day = props.days.find((item) => item.weekIndex === weekIndex && item.weekday === 0)
  if (!day) return ''
  const date = new Date(`${day.date}T12:00:00`)
  return `${date.getMonth() + 1}/${date.getDate()}`
}))

onBeforeUpdate(() => { cellRefs.length = 0 })

function setCellRef(element: Element | null, index: number): void {
  if (element instanceof HTMLButtonElement) cellRefs[index] = element
}

function durationLabel(day: HeatmapDay): string {
  return day.duration ? `${(day.duration / 3_600_000).toFixed(1)} 小时` : '无专注记录'
}

function intensityLabel(value: number): string {
  return ['无记录', '很低', '较低', '中等', '较高', '很高'][value]
}

function ariaLabel(day: HeatmapDay): string {
  return `${day.date} ${weekdayLabels[day.weekday]}，${durationLabel(day)}，强度${intensityLabel(day.intensity)}`
}

function toggleLock(day: HeatmapDay): void {
  locked.value = locked.value?.date === day.date ? null : day
}

function moveFocus(event: KeyboardEvent, day: HeatmapDay): void {
  const index = day.weekIndex * 7 + day.weekday
  const offsets: Record<string, number> = { ArrowLeft: -7, ArrowRight: 7, ArrowUp: -1, ArrowDown: 1 }
  const offset = offsets[event.key]
  if (!offset) return
  event.preventDefault()
  const target = Math.max(0, Math.min(props.days.length - 1, index + offset))
  cellRefs[target]?.focus()
}
</script>

<template>
  <div class="heatmap-calendar">
    <div class="heatmap-week-labels" aria-hidden="true"><span v-for="(label, index) in weekLabels" :key="index">{{ label }}</span></div>
    <div class="heatmap-weekday-labels" aria-hidden="true"><span v-for="label in weekdayLabels" :key="label">{{ label }}</span></div>
    <div class="heatmap-grid" role="grid" aria-label="最近七周专注热力图">
      <button
        v-for="(day, index) in days"
        :key="day.date"
        :ref="(element) => setCellRef(element as Element | null, index)"
        type="button"
        role="gridcell"
        class="heatmap-cell"
        :class="[`intensity-${day.intensity}`, { locked: locked?.date === day.date }]"
        :style="{ gridColumn: day.weekIndex + 1, gridRow: day.weekday + 1 }"
        :aria-label="ariaLabel(day)"
        :aria-pressed="locked?.date === day.date"
        @mouseenter="hovered = day"
        @mouseleave="hovered = null"
        @focus="focused = day"
        @blur="focused = null"
        @click="toggleLock(day)"
        @keydown="moveFocus($event, day)"
      ></button>
    </div>
    <div v-if="active" class="heatmap-tooltip" role="tooltip">
      <strong>{{ active.date }} · {{ weekdayLabels[active.weekday] }}</strong>
      <span>{{ durationLabel(active) }} · {{ intensityLabel(active.intensity) }}</span>
      <small v-if="locked?.date === active.date">已锁定，点击可取消</small>
    </div>
  </div>
</template>
