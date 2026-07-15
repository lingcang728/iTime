<script setup lang="ts">
import { PhCaretLeft, PhCaretRight } from '@phosphor-icons/vue'
import { formatDateLabel } from '../utils/format'
import { useAppStore } from '../stores/appStore'

defineProps<{ title: string; subtitle: string; rangeLabel?: string }>()
const store = useAppStore()
</script>

<template>
  <header class="page-header">
    <div>
      <h1>{{ title }}</h1>
      <p>{{ subtitle }}</p>
    </div>
    <div class="date-switcher" aria-label="日期选择">
      <button class="icon-button" type="button" aria-label="前一天" @click="store.stepDate(-1)"><PhCaretLeft :size="16" /></button>
      <strong>{{ rangeLabel ?? formatDateLabel(store.state.selectedDate) }}</strong>
      <button class="icon-button" type="button" aria-label="后一天" @click="store.stepDate(1)"><PhCaretRight :size="16" /></button>
    </div>
  </header>
</template>

