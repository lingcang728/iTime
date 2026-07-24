import { computed, reactive, ref, shallowRef, watch } from 'vue'
import { mockDates } from '../data/mockEvents'
import type { TimeDataset } from '../domain/events'
import { loadActivityData } from '../providers/activityAdapter'
import { loadProviderActivityData } from '../providers/providerActivityAdapter'
import type { AiToolDetail } from '../providers/prototypeDataProvider'
import { dataProvider, dayRange, EventDataProvider } from '../providers/prototypeDataProvider'
import {
  emptyInputSnapshot,
  inputActivityProvider,
  type InputActivityProvider,
  type InputActivitySnapshot,
} from '../providers/inputActivity'
import { loadKeyboardData } from '../providers/keyboardAdapter'
import {
  getAutostartEnabled,
  refreshDesktopAutostartRegistration,
  setDesktopAutostart,
} from '../platform/autostart'
import { getDesktopRecording, isTauriRuntime, setDesktopRecording } from '../platform/desktop'
import { loadPersistedState, savePersistedState, type PersistedState } from './persistedState'
import { applyDocumentTheme, observeSystemTheme, resolveTheme, systemPrefersDark, type ResolvedTheme, type ThemeMode } from './theme'

export type { ThemeMode } from './theme'
export type MigrationState = 'notFound' | 'partial' | 'ready' | 'imported'
export type ClosePreference = 'ask' | 'hide' | 'quit'

const persisted = loadPersistedState()
const desktopRuntime = isTauriRuntime()
const themeRevision = ref(0)
const requestedTheme = typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('theme')
const previewTheme: ResolvedTheme | undefined = requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined

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
  providerDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'ready' | 'degraded' | 'unavailable',
  providerDataMessage: desktopRuntime ? '正在读取 Codex 与 Claude Code 本机会话' : '浏览器预览数据',
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

const liveActivityDataset = shallowRef<TimeDataset>({ version: 'itime-local-activity-v1', events: [] })
const liveProviderDataset = shallowRef<TimeDataset>({ version: 'itime-local-provider-v1', events: [] })
const liveKeyboardDataset = shallowRef<TimeDataset>({ version: 'itime-keyboard-v1', events: [] })
const liveDataset = computed<TimeDataset>(() => ({
  version: 'itime-local-combined-v1',
  events: [
    ...liveActivityDataset.value.events,
    ...liveProviderDataset.value.events,
    ...liveKeyboardDataset.value.events,
  ],
}))
const runtimeDataProvider = computed(() => desktopRuntime
  ? new EventDataProvider(liveDataset.value)
  : dataProvider)
const day = computed(() => runtimeDataProvider.value.getDay(state.selectedDate))
const week = computed(() => runtimeDataProvider.value.getWeek(state.selectedDate))
const liveInputProvider = shallowRef<InputActivityProvider | null>(null)
const inputDates = shallowRef<string[]>(desktopRuntime ? [] : [...mockDates])
const activityDates = shallowRef<string[]>(desktopRuntime ? [localDate()] : [...mockDates])
const providerDates = shallowRef<string[]>(desktopRuntime ? [] : [...mockDates])

function updateAvailableDates(): void {
  const dates = [...new Set([...inputDates.value, ...activityDates.value, ...providerDates.value])].sort()
  state.availableDates = dates.length ? dates : [localDate()]
  if (!state.availableDates.includes(state.selectedDate)) {
    state.selectedDate = state.availableDates.at(-1) ?? localDate()
  }
}
const input = computed<InputActivitySnapshot>(() => {
  const range = dayRange(state.selectedDate)
  const snapshot = desktopRuntime
    ? liveInputProvider.value?.getSnapshot(range, 'minute') ?? emptyInputSnapshot(range)
    : inputActivityProvider.getSnapshot(range, 'minute')
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
  () => state.quietStart,
  () => state.quietEnd,
  () => ({ ...state.goals }),
  () => state.migrationState,
  () => [...state.deletedInputDates],
], persist, { deep: true })

function applyTheme(preview?: 'light' | 'dark'): void {
  const override = preview ?? previewTheme
  if (override) {
    applyDocumentTheme(override)
    themeRevision.value += 1
    return
  }
  applyDocumentTheme(resolveTheme(state.theme, systemPrefersDark()))
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
let providerRequest = 0
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
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  startDate.setDate(startDate.getDate() - 32)
  try {
    const result = await loadKeyboardData({ start: startDate.getTime(), end: selectedEnd })
    if (request !== inputRequest) return
    const dates = result.provider.getAvailableDates()
    liveInputProvider.value = result.provider
    liveKeyboardDataset.value = result.dataset
    inputDates.value = dates
    updateAvailableDates()
    if (result.snapshot.health.lastError) {
      state.inputDataStatus = 'unavailable'
      state.inputDataMessage = result.snapshot.health.lastError
    } else if (!result.snapshot.health.collectorRunning) {
      state.inputDataStatus = 'unavailable'
      state.inputDataMessage = 'Windows 键盘计数器未运行'
    } else {
      state.inputDataStatus = 'ready'
      state.inputDataMessage = result.snapshot.buckets.length
        ? 'iTime 键盘字符键计数已连接'
        : '键盘计数已启动；从本次版本启用后开始记录'
    }
    state.migrationState = 'ready'
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
    liveActivityDataset.value = result.dataset
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

async function refreshProviderData(): Promise<void> {
  if (!desktopRuntime) return
  const request = ++providerRequest
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  startDate.setDate(startDate.getDate() - 7)
  try {
    const result = await loadProviderActivityData({ start: startDate.getTime(), end: selectedEnd })
    if (request !== providerRequest) return
    liveProviderDataset.value = result.dataset
    providerDates.value = [...new Set(result.dataset.events.map((event) => localDate(new Date(event.start))))].sort()
    updateAvailableDates()
    const connected = result.snapshot.capabilities.codexTaskEvents || result.snapshot.capabilities.claudeTurnEvents
    if (!connected) {
      state.providerDataStatus = 'unavailable'
      state.providerDataMessage = '未找到 Codex 或 Claude Code 的本机会话目录'
    } else if (result.snapshot.skippedFiles > 0) {
      state.providerDataStatus = 'degraded'
      state.providerDataMessage = `Provider 会话已连接；${result.snapshot.skippedFiles} 个记录文件无法读取`
    } else {
      state.providerDataStatus = 'ready'
      state.providerDataMessage = result.snapshot.intervals.length
        ? `已读取 ${result.snapshot.intervals.length} 个本机 Provider 执行区间`
        : 'Provider 会话已连接；所选日期未检测到执行区间'
    }
  } catch (error) {
    if (request !== providerRequest) return
    state.providerDataStatus = 'unavailable'
    state.providerDataMessage = errorMessage(error, 'Codex 与 Claude Code 本机会话暂时不可用')
  }
}

async function refreshAutostart(): Promise<void> {
  if (!desktopRuntime) return
  state.autostartStatus = 'loading'
  try {
    state.autostartEnabled = await getAutostartEnabled()
    if (state.autostartEnabled) {
      state.autostartEnabled = await refreshDesktopAutostartRegistration()
    }
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
    refreshProviderData,
    refreshAutostart,
    setAutostart,
    showToast,
  }
}

observeSystemTheme(
  () => state.theme,
  (theme) => {
    applyDocumentTheme(previewTheme ?? theme)
    themeRevision.value += 1
  },
)

if (desktopRuntime) {
  void refreshInputData()
  void refreshActivityData()
  void refreshProviderData()
  void refreshAutostart()
  watch(() => state.selectedDate, () => {
    void refreshInputData()
    void refreshActivityData()
    void refreshProviderData()
  })
  window.setInterval(() => {
    void refreshInputData()
    void refreshActivityData()
    void refreshProviderData()
  }, 15_000)
}
