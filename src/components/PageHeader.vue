<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { PhCalendarBlank, PhCaretLeft, PhCaretRight } from '@phosphor-icons/vue'
import { formatDateLabel } from '../utils/format'
import { useAppStore } from '../stores/appStore'

const props = defineProps<{ title: string; subtitle: string; rangeLabel?: string }>()
const store = useAppStore()
const selectedIndex = computed(() => store.state.availableDates.indexOf(store.state.selectedDate))
const canGoPrevious = computed(() => selectedIndex.value > 0)
const canGoNext = computed(() => selectedIndex.value >= 0 && selectedIndex.value < store.state.availableDates.length - 1)
const isToday = computed(() => store.isToday.value)
const calendarOpen = ref(false)
const switcherRef = ref<HTMLElement | null>(null)
const visibleMonth = ref(monthStart(store.state.selectedDate))
const weekdays = ['一', '二', '三', '四', '五', '六', '日']

const displayLabel = computed(() => {
  if (props.rangeLabel) return props.rangeLabel
  const base = formatDateLabel(store.state.selectedDate)
  return isToday.value ? `${base} · 今天` : base
})

const available = computed(() => new Set(store.state.availableDates))
const monthLabel = computed(() => {
  const date = visibleMonth.value
  return `${date.getFullYear()}年${String(date.getMonth() + 1).padStart(2, '0')}月`
})
const monthCells = computed(() => {
  const cursor = new Date(visibleMonth.value)
  const lead = (cursor.getDay() + 6) % 7
  cursor.setDate(1 - lead)
  return Array.from({ length: 42 }, () => {
    const date = dateKey(cursor)
    const cell = {
      date,
      day: cursor.getDate(),
      inMonth: cursor.getMonth() === visibleMonth.value.getMonth(),
      enabled: available.value.has(date),
    }
    cursor.setDate(cursor.getDate() + 1)
    return cell
  })
})

watch(() => store.state.selectedDate, (date) => {
  visibleMonth.value = monthStart(date)
})

watch(calendarOpen, (open) => {
  if (open) {
    visibleMonth.value = monthStart(store.state.selectedDate)
    document.addEventListener('pointerdown', onPointerDown)
    return
  }
  document.removeEventListener('pointerdown', onPointerDown)
})

onBeforeUnmount(() => document.removeEventListener('pointerdown', onPointerDown))

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function monthStart(value: string): Date {
  const [year, month] = value.split('-').map(Number)
  return new Date(year || 1970, (month || 1) - 1, 1)
}

function shiftMonth(delta: number): void {
  const next = new Date(visibleMonth.value)
  next.setMonth(next.getMonth() + delta)
  visibleMonth.value = next
}

function chooseDate(date: string): void {
  if (!available.value.has(date)) return
  store.selectDate(date)
  calendarOpen.value = false
}

function onPointerDown(event: PointerEvent): void {
  if (!switcherRef.value?.contains(event.target as Node)) calendarOpen.value = false
}

function todayKey(): string {
  return dateKey(new Date())
}
</script>

<template>
  <header class="page-header">
    <div>
      <h1>{{ title }}</h1>
      <p>{{ subtitle }}</p>
    </div>
    <div ref="switcherRef" class="date-switcher" aria-label="日期选择">
      <button
        class="date-switcher__calendar"
        type="button"
        title="选择日期"
        aria-label="选择日期"
        :aria-expanded="calendarOpen"
        @click="calendarOpen = !calendarOpen"
      >
        <PhCalendarBlank :size="19" weight="regular" aria-hidden="true" />
      </button>
      <div v-if="calendarOpen" class="date-calendar" role="dialog" aria-label="选择日期" @keydown.escape.stop="calendarOpen = false">
        <div class="date-calendar__nav">
          <strong>{{ monthLabel }}</strong>
          <span>
            <button type="button" aria-label="上个月" @click="shiftMonth(-1)"><PhCaretLeft :size="14" aria-hidden="true" /></button>
            <button type="button" aria-label="下个月" @click="shiftMonth(1)"><PhCaretRight :size="14" aria-hidden="true" /></button>
          </span>
        </div>
        <div class="date-calendar__week" aria-hidden="true"><span v-for="day in weekdays" :key="day">{{ day }}</span></div>
        <div class="date-calendar__grid">
          <button
            v-for="cell in monthCells"
            :key="cell.date"
            type="button"
            :disabled="!cell.enabled"
            :class="{ 'is-selected': cell.date === store.state.selectedDate, 'is-today': cell.date === todayKey(), 'is-outside': !cell.inMonth }"
            :aria-pressed="cell.date === store.state.selectedDate"
            @click="chooseDate(cell.date)"
          >{{ cell.day }}</button>
        </div>
        <button class="date-calendar__today" type="button" :disabled="!available.has(todayKey())" @click="store.goToToday(); calendarOpen = false">今天</button>
      </div>
      <strong>{{ displayLabel }}</strong>
      <button v-if="!isToday" class="icon-button date-switcher__today" type="button" @click="store.goToToday()">回到今天</button>
      <button class="icon-button" type="button" aria-label="前一天" :disabled="!canGoPrevious" @click="store.stepDate(-1)"><PhCaretLeft :size="16" aria-hidden="true" /></button>
      <button class="icon-button" type="button" aria-label="后一天" :disabled="!canGoNext" @click="store.stepDate(1)"><PhCaretRight :size="16" aria-hidden="true" /></button>
    </div>
  </header>
</template>
