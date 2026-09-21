<script setup lang="ts">
import { computed } from 'vue'
import { PhCalendarBlank, PhCaretLeft, PhCaretRight } from '@phosphor-icons/vue'
import { formatDateLabel } from '../utils/format'
import { useAppStore } from '../stores/appStore'

const props = defineProps<{ title: string; subtitle: string; rangeLabel?: string }>()
const store = useAppStore()
const selectedIndex = computed(() => store.state.availableDates.indexOf(store.state.selectedDate))
const canGoPrevious = computed(() => selectedIndex.value > 0)
const canGoNext = computed(() => selectedIndex.value >= 0 && selectedIndex.value < store.state.availableDates.length - 1)
const isToday = computed(() => store.isToday.value)

// 选中今天时显式标出；rangeLabel 页面（周报）展示真实区间而非「本周」。
const displayLabel = computed(() => {
  if (props.rangeLabel) return props.rangeLabel
  const base = formatDateLabel(store.state.selectedDate)
  return isToday.value ? `${base} · 今天` : base
})

function pickDate(event: Event): void {
  const input = event.currentTarget
  if (input instanceof HTMLInputElement) store.selectDate(input.value)
}
</script>

<template>
  <header class="page-header">
    <div>
      <h1>{{ title }}</h1>
      <p>{{ subtitle }}</p>
    </div>
    <div class="date-switcher" aria-label="日期选择">
      <label class="date-switcher__calendar date-switcher__calendar--pick" title="选择日期">
        <PhCalendarBlank :size="19" weight="regular" aria-hidden="true" />
        <input
          class="date-switcher__pick"
          type="date"
          :value="store.state.selectedDate"
          :min="store.state.availableDates[0]"
          :max="store.state.availableDates.at(-1)"
          aria-label="选择日期"
          @change="pickDate"
        />
      </label>
      <strong>{{ displayLabel }}</strong>
      <button v-if="!isToday" class="icon-button date-switcher__today" type="button" @click="store.goToToday()">回到今天</button>
      <button class="icon-button" type="button" aria-label="前一天" :disabled="!canGoPrevious" @click="store.stepDate(-1)"><PhCaretLeft :size="16" aria-hidden="true" /></button>
      <button class="icon-button" type="button" aria-label="后一天" :disabled="!canGoNext" @click="store.stepDate(1)"><PhCaretRight :size="16" aria-hidden="true" /></button>
    </div>
  </header>
</template>
