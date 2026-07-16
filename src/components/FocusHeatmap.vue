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
const availableCount = computed(() => props.days.filter((day) => day.available).length)
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
  if (!day.available || day.duration === null) return '未采集'
  return day.duration ? `${(day.duration / 3_600_000).toFixed(1)} 小时` : '已采集，无前台活动'
}

function intensityLabel(day: HeatmapDay): string {
  if (!day.available) return '无可验证数据'
  return ['无活动', '很低', '较低', '中等', '较高', '很高'][day.intensity]
}

function ariaLabel(day: HeatmapDay): string {
  return `${day.date} ${weekdayLabels[day.weekday]}，${durationLabel(day)}，${intensityLabel(day)}`
}

function toggleLock(day: HeatmapDay): void {
  locked.value = locked.value?.date === day.date ? null : day
}

function moveFocus(event: KeyboardEvent, day: HeatmapDay): void {
  const index = day.weekIndex * 7 + day.weekday
  const offsets: Record<string, number> = { ArrowLeft: -7, ArrowRight: 7, ArrowUp: -1, ArrowDown: 1 }
  const offset = offsets[event.key]
  if (offset === undefined) return
  event.preventDefault()
  const target = Math.max(0, Math.min(props.days.length - 1, index + offset))
  cellRefs[target]?.focus()
}
</script>

<template>
  <div class="focus-calendar">
    <div class="focus-calendar__meta"><span>最近七周</span><strong>{{ availableCount }} 天有本机记录</strong></div>
    <div class="focus-calendar__body">
      <div class="focus-calendar__week-labels" aria-hidden="true"><span v-for="(label, index) in weekLabels" :key="index">{{ label }}</span></div>
      <div class="focus-calendar__day-labels" aria-hidden="true"><span v-for="label in weekdayLabels" :key="label">{{ label }}</span></div>
      <div class="focus-calendar__grid" role="grid" aria-label="最近七周专注热力图">
        <button
          v-for="(day, index) in days"
          :key="day.date"
          :ref="(element) => setCellRef(element as Element | null, index)"
          type="button"
          role="gridcell"
          class="focus-calendar__cell heatmap-cell"
          :class="[`intensity-${day.intensity}`, { 'is-locked': locked?.date === day.date, locked: locked?.date === day.date, 'is-unavailable': !day.available }]"
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
      <div v-if="active" class="focus-calendar__tooltip heatmap-tooltip" role="tooltip">
        <strong>{{ active.date }} · {{ weekdayLabels[active.weekday] }}</strong>
        <span>{{ durationLabel(active) }} · {{ intensityLabel(active) }}</span>
        <small v-if="locked?.date === active.date">已锁定，点击可取消</small>
      </div>
    </div>
    <div class="focus-calendar__scale"><span>低</span><i></i><span>高</span></div>
  </div>
</template>

<style scoped>
.focus-calendar { --heat-0: #edf2ee; --heat-1: #d9eee0; --heat-2: #bce2ca; --heat-3: #8ed0a7; --heat-4: #57b77d; --heat-5: #26955f; min-width: 0; }
.focus-calendar__meta { display: flex; justify-content: space-between; color: var(--text-muted); font-size: 9px; }
.focus-calendar__meta strong { color: var(--accent-green-strong); font-weight: 650; }
.focus-calendar__body { min-width: 0; position: relative; display: grid; grid-template-columns: 30px minmax(0, 1fr); grid-template-rows: 16px minmax(0, 1fr); gap: 5px 7px; margin-top: 10px; }
.focus-calendar__week-labels { grid-column: 2; display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 5px; color: var(--text-muted); font-size: 8px; text-align: center; }
.focus-calendar__day-labels { grid-row: 2; display: grid; grid-template-rows: repeat(7, 1fr); align-items: center; color: var(--text-muted); font-size: 8px; }
.focus-calendar__grid { min-width: 0; grid-column: 2; grid-row: 2; display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); grid-template-rows: repeat(7, minmax(0, 1fr)); gap: 5px; }
.focus-calendar__cell { min-width: 0; height: 23px; padding: 0; border: 1px solid transparent; border-radius: 5px; background: var(--heat-0); cursor: pointer; transition: transform 140ms ease, box-shadow 140ms ease, border-color 140ms ease; }
.focus-calendar__cell:hover,
.focus-calendar__cell:focus-visible { transform: translateY(-1px) scale(1.04); outline: none; box-shadow: 0 4px 12px color-mix(in srgb, #26955f 24%, transparent); }
.focus-calendar__cell.is-locked { border-color: #1d6f49; box-shadow: 0 0 0 2px var(--bg-card), 0 0 0 3px #3aa66d; }
.focus-calendar__cell.is-unavailable { background: var(--heat-0); opacity: .58; }
.focus-calendar__cell.intensity-1 { background: var(--heat-1); }
.focus-calendar__cell.intensity-2 { background: var(--heat-2); }
.focus-calendar__cell.intensity-3 { background: var(--heat-3); }
.focus-calendar__cell.intensity-4 { background: var(--heat-4); }
.focus-calendar__cell.intensity-5 { background: var(--heat-5); }
.focus-calendar__tooltip { min-width: 164px; position: absolute; z-index: 4; right: 4px; bottom: 4px; display: grid; gap: 2px; padding: 8px 10px; border: 1px solid var(--border-soft); border-radius: 9px; color: var(--text-primary); background: color-mix(in srgb, var(--bg-card) 96%, transparent); box-shadow: var(--shadow-card); font-size: 9px; pointer-events: none; }
.focus-calendar__tooltip span,
.focus-calendar__tooltip small { color: var(--text-secondary); font-size: 8px; }
.focus-calendar__scale { display: flex; align-items: center; gap: 8px; margin-top: 10px; color: var(--text-muted); font-size: 8px; }
.focus-calendar__scale i { flex: 1; height: 6px; border-radius: 4px; background: linear-gradient(90deg, var(--heat-0), var(--heat-1), var(--heat-2), var(--heat-3), var(--heat-4), var(--heat-5)); }
:global(html[data-theme="dark"]) .focus-calendar { --heat-0: #202a24; --heat-1: #243b2d; --heat-2: #2d583d; --heat-3: #34734c; --heat-4: #3b905c; --heat-5: #4eb578; }
</style>
