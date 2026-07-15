import { computed, reactive, watch } from 'vue'
import { mockDates } from '../data/mockEvents'
import type { AiToolDetail } from '../providers/prototypeDataProvider'
import { dataProvider, dayRange } from '../providers/prototypeDataProvider'
import { inputActivityProvider, type InputActivitySnapshot } from '../providers/inputActivity'
import { setDesktopRecording } from '../platform/desktop'

export type ThemeMode = 'light' | 'dark' | 'system'
export type MigrationState = 'notFound' | 'partial' | 'ready' | 'imported'
export type ClosePreference = 'ask' | 'hide' | 'quit'

interface PersistedState {
  theme: ThemeMode
  recording: boolean
  reminders: boolean
  hideToTray: boolean
  closePreference: ClosePreference
  showInputDensity: boolean
  heatmapEnabled: boolean
  shortcutsEnabled: boolean
  goals: Record<string, number>
  migrationState: MigrationState
  deletedInputDates: string[]
}

const defaults: PersistedState = {
  theme: 'light',
  recording: true,
  reminders: true,
  hideToTray: true,
  closePreference: 'ask',
  showInputDensity: false,
  heatmapEnabled: true,
  shortcutsEnabled: true,
  goals: { learning: 120, development: 180, ai: 180, continuous: 50 },
  migrationState: 'partial',
  deletedInputDates: [],
}

function loadPersisted(): PersistedState {
  if (typeof localStorage === 'undefined') return defaults
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem('itime-prototype-state') ?? '{}') }
  } catch {
    return defaults
  }
}

const persisted = loadPersisted()
const state = reactive({
  selectedDate: mockDates[mockDates.length - 1],
  selectedToolId: null as string | null,
  detailDrawerOpen: false,
  closeDialogOpen: false,
  rememberCloseChoice: false,
  toast: '',
  ...persisted,
})

const day = computed(() => dataProvider.getDay(state.selectedDate))
const week = computed(() => dataProvider.getWeek(state.selectedDate))
const input = computed<InputActivitySnapshot>(() => {
  const snapshot = inputActivityProvider.getSnapshot(dayRange(state.selectedDate), 'hour')
  if (!state.deletedInputDates.includes(state.selectedDate)) return snapshot
  return {
    ...snapshot,
    cumulative: { ...snapshot.cumulative, keyStrokes: 0, leftClicks: 0, rightClicks: 0, mouseDistance: 0, scrollDistance: 0 },
    history: [],
    singleKeys: [],
    shortcuts: [],
  }
})
const selectedTool = computed<AiToolDetail | null>(() => state.selectedToolId ? dataProvider.getToolDetail(state.selectedDate, state.selectedToolId) : null)

function persist(): void {
  if (typeof localStorage === 'undefined') return
  const value: PersistedState = {
    theme: state.theme,
    recording: state.recording,
    reminders: state.reminders,
    hideToTray: state.hideToTray,
    closePreference: state.closePreference,
    showInputDensity: state.showInputDensity,
    heatmapEnabled: state.heatmapEnabled,
    shortcutsEnabled: state.shortcutsEnabled,
    goals: state.goals,
    migrationState: state.migrationState,
    deletedInputDates: state.deletedInputDates,
  }
  localStorage.setItem('itime-prototype-state', JSON.stringify(value))
}

watch(state, persist, { deep: true })

function applyTheme(): void {
  const systemDark = typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches
  const resolved = state.theme === 'system' ? (systemDark ? 'dark' : 'light') : state.theme
  document.documentElement.dataset.theme = resolved
}

watch(() => state.theme, applyTheme)

function stepDate(delta: number): void {
  const index = mockDates.indexOf(state.selectedDate)
  state.selectedDate = mockDates[Math.max(0, Math.min(mockDates.length - 1, index + delta))]
}

function openTool(toolId: string): void {
  state.selectedToolId = toolId
  state.detailDrawerOpen = true
}

function closeTool(): void {
  state.detailDrawerOpen = false
}

async function setRecording(recording: boolean): Promise<void> {
  state.recording = recording
  await setDesktopRecording(recording)
}

function deleteInputDate(date: string): void {
  if (!state.deletedInputDates.includes(date)) state.deletedInputDates.push(date)
  state.toast = `已删除 ${date} 的输入统计演示数据`
}

function showToast(message: string): void {
  state.toast = message
  window.setTimeout(() => {
    if (state.toast === message) state.toast = ''
  }, 2600)
}

export function useAppStore() {
  return {
    state,
    day,
    week,
    input,
    selectedTool,
    stepDate,
    openTool,
    closeTool,
    setRecording,
    applyTheme,
    deleteInputDate,
    showToast,
  }
}

