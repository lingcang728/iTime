import { computed, reactive, ref, shallowRef, watch } from 'vue'
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
import { getAutostartEnabled, setDesktopAutostart } from '../platform/autostart'
import { getDesktopRecording, isTauriRuntime, setDesktopRecording } from '../platform/desktop'
import { loadPersistedState, savePersistedState, type PersistedState } from './persistedState'

export type ThemeMode = 'light' | 'dark' | 'system'
export type MigrationState = 'notFound' | 'partial' | 'ready' | 'imported'
export type ClosePreference = 'ask' | 'hide' | 'quit'

const persisted = loadPersistedState()
const desktopRuntime = isTauriRuntime()
const themeRevision = ref(0)

function localDate(value = new Date()): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

const state = reactive({
  selectedDate: desktopRuntime ? localDate() : mockDates[mockDates.length - 1],
  availableDates: desktopRuntime ? [localDate()] : [...mockDates],
  inputDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'ready' | 'unavailable',
  inputDataMessage: desktopRuntime ? '正在读取 iTime 本机输入记录' : '浏览器预览数据',
  activityDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'ready' | 'degraded' | 'unavailable',
  activityDataMessage: desktopRuntime ? '正在读取 iTime 本机活动记录' : '浏览器预览数据',
  autostartEnabled: false,
  autostartStatus: (desktopRuntime ? 'loading' : 'ready') as 'loading' | 'ready' | 'error',
  autostartMessage: desktopRuntime ? '正在读取 Windows 启动设置' : '仅桌面版可设置开机自启动',
  selectedToolId: null as string | null,
  detailDrawerOpen: false,
  closeDialogOpen: false,
  rememberCloseChoice: false,
  toast: '',
  ...persisted,
  recording: true,
})

const liveDataset = shallowRef<TimeDataset>({ version: 'itime-local-activity-v1', events: [] })
const runtimeDataProvider = computed(() => desktopRuntime
  ? new EventDataProvider(liveDataset.value)
  : dataProvider)
const day = computed(() => runtimeDataProvider.value.getDay(state.selectedDate))
const week = computed(() => runtimeDataProvider.value.getWeek(state.selectedDate))
const liveInputProvider = shallowRef<InputActivityProvider | null>(null)
const inputDates = shallowRef<string[]>(desktopRuntime ? [] : [...mockDates])
const activityDates = shallowRef<string[]>(desktopRuntime ? [localDate()] : [...mockDates])

function updateAvailableDates(): void {
  const dates = [...new Set([...inputDates.value, ...activityDates.value])].sort()
  state.availableDates = dates.length ? dates : [localDate()]
  if (!state.availableDates.includes(state.selectedDate)) {
    state.selectedDate = state.availableDates.at(-1) ?? localDate()
  }
}
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
    schemaVersion: state.schemaVersion,
    theme: state.theme,
    reminders: state.reminders,
    closePreference: state.closePreference,
    heatmapEnabled: state.heatmapEnabled,
    shortcutsEnabled: state.shortcutsEnabled,
    quietStart: state.quietStart,
    quietEnd: state.quietEnd,
    goals: state.goals,
    migrationState: state.migrationState,
    deletedInputDates: state.deletedInputDates,
  }
  savePersistedState(value)
}

watch([
  () => state.theme,
  () => state.reminders,
  () => state.closePreference,
  () => state.heatmapEnabled,
  () => state.shortcutsEnabled,
  () => state.quietStart,
  () => state.quietEnd,
  () => ({ ...state.goals }),
  () => state.migrationState,
  () => [...state.deletedInputDates],
], persist, { deep: true })

function applyTheme(preview?: 'light' | 'dark'): void {
  if (preview) {
    document.documentElement.dataset.theme = preview
    themeRevision.value += 1
    return
  }
  const systemDark = typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches
  const resolved = state.theme === 'system' ? (systemDark ? 'dark' : 'light') : state.theme
  document.documentElement.dataset.theme = resolved
  themeRevision.value += 1
}

watch(() => state.theme, () => applyTheme())

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
  try {
    await setDesktopRecording(recording)
    state.recording = recording
  } catch (error) {
    showToast(errorMessage(error, '无法修改活动记录状态'))
  }
}

async function syncRecording(): Promise<void> {
  try {
    state.recording = await getDesktopRecording()
  } catch (error) {
    showToast(errorMessage(error, '无法读取活动记录状态'))
  }
}

function deleteInputDate(date: string): void {
  if (!input.value.capabilities.deleteByDate) {
    showToast('本机输入历史为只读记录，iTime 不会修改它')
    return
  }
  if (!state.deletedInputDates.includes(date)) state.deletedInputDates.push(date)
  showToast(`已删除 ${date} 的输入统计`)
}

let inputRequest = 0
let activityRequest = 0
let toastRequest = 0

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message
  if (typeof error === 'string' && error) return error
  if (error && typeof error === 'object') {
    const candidate = error as { message?: unknown; error?: unknown }
    if (typeof candidate.message === 'string') return candidate.message
    if (typeof candidate.error === 'string') return candidate.error
    try { return JSON.stringify(error) } catch { return fallback }
  }
  return fallback
}

async function refreshInputData(): Promise<void> {
  if (!desktopRuntime) return
  const request = ++inputRequest
  try {
    const provider = await loadKeyStatsProvider()
    if (request !== inputRequest) return
    const dates = provider.getAvailableDates()
    liveInputProvider.value = provider
    inputDates.value = dates
    updateAvailableDates()
    state.inputDataStatus = 'ready'
    state.inputDataMessage = 'iTime 本机输入记录已连接'
    state.migrationState = 'imported'
  } catch (error) {
    if (request !== inputRequest) return
    state.inputDataStatus = 'unavailable'
    state.inputDataMessage = errorMessage(error, '本机输入数据暂时不可用')
  }
}

async function refreshActivityData(): Promise<void> {
  if (!desktopRuntime) return
  const request = ++activityRequest
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  startDate.setDate(startDate.getDate() - 7)
  try {
    const result = await loadActivityData({ start: startDate.getTime(), end: selectedEnd })
    if (request !== activityRequest) return
    liveDataset.value = result.dataset
    activityDates.value = [...new Set([
      ...activityDates.value,
      ...result.dataset.events.map((event) => localDate(new Date(event.start))),
    ])].sort()
    updateAvailableDates()
    if (result.snapshot.health.lastError) {
      state.activityDataStatus = 'degraded'
      state.activityDataMessage = `采集写入异常：${result.snapshot.health.lastError}`
    } else if (result.snapshot.skippedRecords > 0) {
      state.activityDataStatus = 'degraded'
      state.activityDataMessage = `已跳过 ${result.snapshot.skippedRecords} 条损坏或不兼容的活动记录`
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
    if (request !== activityRequest) return
    state.activityDataStatus = 'unavailable'
    state.activityDataMessage = errorMessage(error, 'iTime 本机活动记录暂时不可用')
  }
}

async function refreshAutostart(): Promise<void> {
  if (!desktopRuntime) return
  state.autostartStatus = 'loading'
  try {
    state.autostartEnabled = await getAutostartEnabled()
    state.autostartStatus = 'ready'
    state.autostartMessage = state.autostartEnabled ? '已由 Windows 注册开机启动' : '当前不会随 Windows 启动'
  } catch (error) {
    state.autostartStatus = 'error'
    state.autostartMessage = errorMessage(error, '无法读取 Windows 启动设置')
  }
}

async function setAutostart(enabled: boolean): Promise<void> {
  if (!desktopRuntime) return
  state.autostartStatus = 'loading'
  try {
    const confirmed = await setDesktopAutostart(enabled)
    state.autostartEnabled = confirmed
    state.autostartStatus = confirmed === enabled ? 'ready' : 'error'
    state.autostartMessage = confirmed === enabled
      ? (confirmed ? '已开启；下次登录 Windows 时自动启动' : '已关闭开机自启动')
      : 'Windows 返回的启动状态与请求不一致'
  } catch (error) {
    state.autostartStatus = 'error'
    state.autostartMessage = errorMessage(error, '无法修改 Windows 启动设置')
  }
}

function showToast(message: string): void {
  const request = ++toastRequest
  state.toast = message
  window.setTimeout(() => {
    if (request === toastRequest) state.toast = ''
  }, 2600)
}

export function useAppStore() {
  return {
    state,
    themeRevision,
    day,
    week,
    input,
    selectedTool,
    stepDate,
    openTool,
    closeTool,
    setRecording,
    syncRecording,
    applyTheme,
    deleteInputDate,
    refreshInputData,
    refreshActivityData,
    refreshAutostart,
    setAutostart,
    showToast,
  }
}

if (typeof matchMedia !== 'undefined') {
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (state.theme === 'system') applyTheme()
  })
}

if (desktopRuntime) {
  void refreshInputData()
  void refreshActivityData()
  void refreshAutostart()
  watch(() => state.selectedDate, () => void refreshActivityData())
  window.setInterval(() => {
    void refreshInputData()
    void refreshActivityData()
  }, 15_000)
}
