import { computed, reactive, shallowRef, watch } from 'vue'
import { mockDates } from '../data/mockEvents'
import type { TimeDataset } from '../domain/events'
import { loadActivityData } from '../providers/activityAdapter'
import type { AiToolDetail } from '../providers/prototypeDataProvider'
import { dataProvider, dayRange, EventDataProvider } from '../providers/prototypeDataProvider'
import {
  emptyInputSnapshot,
  inputActivityProvider,
  type InputActivityProvider,
  type InputActivitySnapshot,
} from '../providers/inputActivity'
import { loadKeyStatsProvider } from '../providers/keyStatsAdapter'
import { isTauriRuntime, setDesktopRecording } from '../platform/desktop'

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
const desktopRuntime = isTauriRuntime()

function localDate(value = new Date()): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

const state = reactive({
  selectedDate: desktopRuntime ? localDate() : mockDates[mockDates.length - 1],
  availableDates: desktopRuntime ? [localDate()] : [...mockDates],
  inputDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'ready' | 'unavailable',
  inputDataMessage: desktopRuntime ? '正在读取本机 KeyStats 聚合记录' : '浏览器预览数据',
  activityDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'ready' | 'degraded' | 'unavailable',
  activityDataMessage: desktopRuntime ? '正在读取 iTime 本机活动记录' : '浏览器预览数据',
  selectedToolId: null as string | null,
  detailDrawerOpen: false,
  closeDialogOpen: false,
  rememberCloseChoice: false,
  toast: '',
  ...persisted,
})

const liveDataset = shallowRef<TimeDataset>({ version: 'itime-local-activity-v1', events: [] })
const runtimeDataProvider = computed(() => desktopRuntime
  ? new EventDataProvider(liveDataset.value)
  : dataProvider)
const day = computed(() => runtimeDataProvider.value.getDay(state.selectedDate))
const week = computed(() => runtimeDataProvider.value.getWeek(state.selectedDate))
const liveInputProvider = shallowRef<InputActivityProvider | null>(null)
const input = computed<InputActivitySnapshot>(() => {
  const range = dayRange(state.selectedDate)
  const snapshot = desktopRuntime
    ? liveInputProvider.value?.getSnapshot(range, 'hour') ?? emptyInputSnapshot(range)
    : inputActivityProvider.getSnapshot(range, 'hour')
  if (!state.deletedInputDates.includes(state.selectedDate)) return snapshot
  return {
    ...snapshot,
    cumulative: {
      ...snapshot.cumulative,
      keyStrokes: 0,
      leftClicks: null,
      rightClicks: null,
      combinedClicks: 0,
      mouseDistance: 0,
      scrollDistance: 0,
    },
    history: [],
    singleKeys: [],
    shortcuts: [],
  }
})
const selectedTool = computed<AiToolDetail | null>(() => state.selectedToolId
  ? runtimeDataProvider.value.getToolDetail(state.selectedDate, state.selectedToolId)
  : null)

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
  const index = state.availableDates.indexOf(state.selectedDate)
  const next = Math.max(0, Math.min(state.availableDates.length - 1, index + delta))
  state.selectedDate = state.availableDates[next] ?? state.selectedDate
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
  if (!input.value.capabilities.deleteByDate) {
    showToast('KeyStats 是只读数据源，iTime 不会修改它的历史记录')
    return
  }
  if (!state.deletedInputDates.includes(date)) state.deletedInputDates.push(date)
  showToast(`已删除 ${date} 的输入统计`)
}

async function refreshInputData(): Promise<void> {
  if (!desktopRuntime) return
  try {
    const provider = await loadKeyStatsProvider()
    const dates = provider.getAvailableDates()
    liveInputProvider.value = provider
    state.availableDates = dates
    if (!dates.includes(state.selectedDate)) state.selectedDate = dates.at(-1) ?? localDate()
    state.inputDataStatus = 'ready'
    state.inputDataMessage = '已连接 KeyStats 本机只读记录'
    state.migrationState = 'imported'
  } catch (error) {
    state.inputDataStatus = 'unavailable'
    state.inputDataMessage = error instanceof Error ? error.message : 'KeyStats 数据暂时不可用'
  }
}

async function refreshActivityData(): Promise<void> {
  if (!desktopRuntime) return
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  startDate.setDate(startDate.getDate() - 7)
  try {
    const result = await loadActivityData({ start: startDate.getTime(), end: selectedEnd })
    liveDataset.value = result.dataset
    if (result.snapshot.health.lastError) {
      state.activityDataStatus = 'degraded'
      state.activityDataMessage = `采集写入异常：${result.snapshot.health.lastError}`
    } else if (!result.snapshot.health.collectorRunning) {
      state.activityDataStatus = 'unavailable'
      state.activityDataMessage = '本机活动采集器未运行'
    } else {
      state.activityDataStatus = 'ready'
      state.activityDataMessage = result.snapshot.intervals.length
        ? '本机活动已连接；仅统计启用后的记录'
        : '已开始记录；接入前的应用历史不会补造'
    }
  } catch (error) {
    state.activityDataStatus = 'unavailable'
    state.activityDataMessage = error instanceof Error ? error.message : 'iTime 本机活动记录暂时不可用'
  }
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
    refreshInputData,
    refreshActivityData,
    showToast,
  }
}

if (desktopRuntime) {
  void refreshInputData()
  void refreshActivityData()
  watch(() => state.selectedDate, () => void refreshActivityData())
  window.setInterval(() => {
    void refreshInputData()
    void refreshActivityData()
  }, 15_000)
}
