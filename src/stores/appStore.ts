import { computed, reactive, ref, shallowRef, watch } from 'vue'
import { mockDates } from '../data/mockEvents'
import type { DaySnapshot, TimeDataset } from '../domain/events'
import { loadActivityData } from '../providers/activityAdapter'
import {
  defaultProviderConsent,
  loadProviderActivityData,
  loadProviderConsent,
  saveProviderConsent,
  type ProviderConsent,
} from '../providers/providerActivityAdapter'
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
  setDesktopAutostart,
} from '../platform/autostart'
import { getDesktopRecording, isTauriRuntime, setDesktopRecording } from '../platform/desktop'
import {
  clearAllLocalData,
  exportLocalData,
  loadLocalDataStatus,
  openLocalDataDirectory,
  saveDataRetention,
  type DataExportFormat,
  type DataRetentionDays,
  type LocalDataStatus,
} from '../platform/localData'
import type { ActivityDataStatus, InputDataStatus } from './dataAvailability'
import { loadPersistedState, savePersistedState, type PersistedState } from './persistedState'
import { applyDocumentTheme, observeSystemTheme, resolveTheme, systemPrefersDark, type ResolvedTheme, type ThemeMode } from './theme'

export type { ThemeMode } from './theme'
export type ClosePreference = 'ask' | 'hide' | 'quit'
export interface ReminderOccurrence {
  occurrenceId: string
  continuousMinutes: number
}

const previewLocalData: LocalDataStatus = {
  directory: '仅桌面版显示实际数据目录',
  retentionDays: null,
  fileCount: 0,
  sizeBytes: 0,
  lastWriteAt: null,
  activityRecords: 0,
  keyboardRecords: 0,
  skippedRecords: 0,
  startAt: null,
  endAt: null,
  health: 'empty',
  message: '浏览器预览不会读取、导出或删除本机数据',
}

const persisted = loadPersistedState()
const desktopRuntime = isTauriRuntime()
const themeRevision = ref(0)
const requestedTheme = typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('theme')
const previewTheme: ResolvedTheme | undefined = requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined
const previewProviderConsent: ProviderConsent = {
  ...defaultProviderConsent,
  noticeSeen: true,
  aiAgentToolsEnabled: true,
}

function localDate(value = new Date()): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

const state = reactive({
  selectedDate: desktopRuntime ? localDate() : mockDates[mockDates.length - 1],
  availableDates: desktopRuntime ? [localDate()] : [...mockDates],
  inputDataStatus: (desktopRuntime ? 'loading' : 'preview') as InputDataStatus,
  inputDataMessage: desktopRuntime ? '读取中' : '预览数据',
  activityDataStatus: (desktopRuntime ? 'loading' : 'preview') as ActivityDataStatus,
  activityDataMessage: desktopRuntime ? '读取中' : '预览数据',
  providerDataStatus: (desktopRuntime ? 'disabled' : 'preview') as ActivityDataStatus,
  providerDataMessage: desktopRuntime ? '未授权' : '预览数据',
  providerConsent: desktopRuntime ? { ...defaultProviderConsent } : previewProviderConsent,
  providerConsentStatus: (desktopRuntime ? 'loading' : 'ready') as 'loading' | 'ready' | 'error',
  lastDataRefreshAt: desktopRuntime ? null as number | null : Date.now(),
  autostartEnabled: false,
  autostartStatus: (desktopRuntime ? 'loading' : 'ready') as 'loading' | 'ready' | 'error',
  autostartMessage: desktopRuntime ? '读取中' : '仅桌面版',
  localDataStatus: (desktopRuntime ? 'loading' : 'preview') as 'loading' | 'preview' | 'empty' | 'degraded' | 'error' | 'ready',
  localDataMessage: desktopRuntime ? '检查中' : previewLocalData.message,
  localData: { ...previewLocalData },
  localDataBusy: null as null | 'open' | 'json' | 'csv' | 'retention' | 'clear',
  localDataExportMessage: '',
  selectedToolId: null as string | null,
  detailDrawerOpen: false,
  closeDialogOpen: false,
  toast: '',
  toastTone: 'default' as 'default' | 'error',
  ...persisted,
  recording: true,
  recordingStatus: (desktopRuntime ? 'loading' : 'ready') as 'loading' | 'ready' | 'error',
  recordingMessage: desktopRuntime ? '确认中' : '预览不写入',
  currentReminder: null as ReminderOccurrence | null,
  // P3-26: reminder config sync runs at startup and on every toggle; a failed
  // push leaves the backend out of sync with the toggle, so the failure must
  // stay visible next to the control — not just a transient toast.
  reminderSyncFailed: false,
  reminderSyncMessage: '',
})

const liveActivityDataset = shallowRef<TimeDataset>({ version: 'itime-local-activity-v1', events: [] })
const liveProviderDataset = shallowRef<TimeDataset>({ version: 'itime-local-provider-v1', events: [] })
const liveKeyboardDataset = shallowRef<TimeDataset>({ version: 'itime-keyboard-v1', events: [] })
// P1-2: stable reference — only re-merges when a source actually changes
const liveDataset = shallowRef<TimeDataset>({ version: 'itime-local-combined-v1', events: [] })
watch(
  [liveActivityDataset, liveProviderDataset, liveKeyboardDataset],
  ([a, p, k]) => {
    liveDataset.value = {
      version: 'itime-local-combined-v1',
      events: [...a.events, ...p.events, ...k.events],
    }
  },
  { flush: 'sync', immediate: true },
)

const runtimeDataProvider = computed(() => desktopRuntime
  ? new EventDataProvider(liveDataset.value)
  : dataProvider)

// P1-1: snapshot cache — invalidated on every data refresh, O(1) on repeated reads
const snapshotCache = new Map<string, DaySnapshot>()
watch(runtimeDataProvider, () => { snapshotCache.clear() }, { flush: 'sync' })

function localDateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function getCachedDay(date: string): DaySnapshot {
  if (!snapshotCache.has(date)) snapshotCache.set(date, runtimeDataProvider.value.getDay(date))
  return snapshotCache.get(date)!
}

const day = computed(() => getCachedDay(state.selectedDate))
const week = computed(() => {
  const end = new Date(`${state.selectedDate}T12:00:00`)
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(end)
    d.setDate(end.getDate() - (6 - i))
    return getCachedDay(localDateKey(d))
  })
})
/** Rolling 7 days ending the day before `week` starts — the previous-period baseline. */
const previousWeek = computed(() => {
  const end = new Date(`${state.selectedDate}T12:00:00`)
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(end)
    d.setDate(end.getDate() - (13 - i))
    return getCachedDay(localDateKey(d))
  })
})
const isToday = computed(() => state.selectedDate === localDate())
const liveInputProvider = shallowRef<InputActivityProvider | null>(null)
const inputDates = shallowRef<string[]>(desktopRuntime ? [] : [...mockDates])
const activityDates = shallowRef<string[]>(desktopRuntime ? [localDate()] : [...mockDates])
const providerDates = shallowRef<string[]>(desktopRuntime ? [] : [...mockDates])

/**
 * Free date navigation bound. `localData.startAt` (epoch ms over all retained
 * activity+keyboard shards) is the authoritative earliest recorded day — it
 * moves forward when retention/clear actually deletes data, so dates do not
 * linger in the switcher. AI-agent sessions live outside the Data directory,
 * so provider dates join the range separately. Days without records stay
 * navigable: "no data that day" is a page empty state, not unreachable UI.
 */
const MAX_NAVIGABLE_DAYS = 731

function enumerateLocalDates(from: string, to: string): string[] {
  const start = new Date(`${from}T00:00:00`)
  const end = new Date(`${to}T00:00:00`)
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime()) || start > end) return []
  const dates: string[] = []
  const cursor = new Date(start)
  while (cursor <= end && dates.length < MAX_NAVIGABLE_DAYS) {
    dates.push(localDate(cursor))
    cursor.setDate(cursor.getDate() + 1)
  }
  return dates
}

function updateAvailableDates(): void {
  const known = new Set([...inputDates.value, ...activityDates.value, ...providerDates.value])
  const today = localDate()
  const sortedKnown = [...known].sort()
  const recordedStart = state.localData.startAt
  const recordedEarliest = recordedStart !== null && Number.isFinite(recordedStart)
    ? localDate(new Date(recordedStart))
    : null
  const earliest = [recordedEarliest, sortedKnown[0], today]
    .filter((value): value is string => Boolean(value))
    .sort()[0] ?? today
  const latest = sortedKnown.at(-1) && sortedKnown.at(-1)! > today ? sortedKnown.at(-1)! : today
  const dates = new Set(enumerateLocalDates(earliest > latest ? today : earliest, latest))
  for (const date of known) dates.add(date)
  state.availableDates = dates.size ? [...dates].sort() : [today]
  if (!state.availableDates.includes(state.selectedDate)) {
    // Clamp to the nearest bound rather than jumping back to today.
    state.selectedDate = state.selectedDate < state.availableDates[0]!
      ? state.availableDates[0]!
      : state.availableDates.at(-1)!
  }
}
const input = computed<InputActivitySnapshot>(() => {
  const range = dayRange(state.selectedDate)
  return desktopRuntime
    ? liveInputProvider.value?.getSnapshot(range, 'minute') ?? emptyInputSnapshot(range)
    : inputActivityProvider.getSnapshot(range, 'minute')
})
const inputHistory = computed<InputActivitySnapshot>(() => {
  const selectedRange = dayRange(state.selectedDate)
  const historyStart = new Date(selectedRange.start)
  historyStart.setDate(historyStart.getDate() - 29)
  const range = { start: historyStart.getTime(), end: selectedRange.end }
  const provider = desktopRuntime ? liveInputProvider.value : inputActivityProvider
  return provider?.getSnapshot(range, 'day') ?? emptyInputSnapshot(range)
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
    dismissedReminderOccurrences: state.dismissedReminderOccurrences,
    rememberCloseChoice: state.rememberCloseChoice,
  }
  savePersistedState(value)
}

const stopPersistWatch = watch([
  () => state.theme,
  () => state.reminders,
  () => state.closePreference,
  () => state.quietStart,
  () => state.quietEnd,
  () => ({ ...state.goals }),
  () => [...state.dismissedReminderOccurrences],
  () => state.rememberCloseChoice,
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

const stopThemeWatch = watch(() => state.theme, () => applyTheme())

function goToToday(): void {
  const today = localDate()
  if (!state.availableDates.includes(today)) {
    state.availableDates = [...state.availableDates, today].sort()
  }
  state.selectedDate = today
}

/**
 * Direct date pick (native <input type="date">). `availableDates` enumerates
 * every day between the earliest retained record and today, so any in-range
 * pick lands on a navigable day; out-of-range picks clamp to the bound.
 * Selecting an old day re-issues a bounded snapshot query via the
 * selectedDate watcher — recorded history is reachable, not just exportable.
 */
function selectDate(date: string): void {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return
  if (Number.isNaN(new Date(`${date}T12:00:00`).getTime())) return
  const first = state.availableDates[0]
  const last = state.availableDates.at(-1)
  if (first && date < first) date = first
  if (last && date > last) date = last
  if (date !== state.selectedDate) state.selectedDate = date
}

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
  if (state.recordingStatus === 'loading') return
  const previous = state.recording
  state.recordingStatus = 'loading'
  state.recordingMessage = recording ? '启动中' : '暂停中'
  try {
    state.recording = await setDesktopRecording(recording)
    state.recordingStatus = 'ready'
    state.recordingMessage = state.recording ? '记录中' : '已暂停'
  } catch (error) {
    state.recording = previous
    state.recordingStatus = 'error'
    state.recordingMessage = errorMessage(error, '无法修改')
    showToast(state.recordingMessage, 'error')
  }
}

async function syncRecording(): Promise<void> {
  state.recordingStatus = 'loading'
  try {
    state.recording = await getDesktopRecording()
    state.recordingStatus = 'ready'
    state.recordingMessage = state.recording ? '记录中' : '已暂停'
  } catch (error) {
    state.recordingStatus = 'error'
    state.recordingMessage = errorMessage(error, '状态读取失败')
    showToast(state.recordingMessage, 'error')
  }
}

let inputRequest = 0
let activityRequest = 0
let providerRequest = 0
let localDataRequest = 0
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
  state.inputDataStatus = 'loading'
  state.inputDataMessage = '读取中'
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
    const { health } = result.snapshot
    if (!health.collectorRunning) {
      state.inputDataStatus = 'error'
      state.inputDataMessage = '键盘计数器未运行；请尝试重启应用'
    } else if (!health.writerRunning || health.queueDisconnected) {
      state.inputDataStatus = 'error'
      state.inputDataMessage = health.lastError || '写入线程未运行'
    } else if (health.droppedEvents > 0) {
      state.inputDataStatus = 'degraded'
      state.inputDataMessage = `已丢弃 ${health.droppedEvents} 次输入事件`
    } else if (health.writeFailures > 0 || health.readFailures > 0 || result.snapshot.skippedRecords > 0) {
      state.inputDataStatus = 'degraded'
      state.inputDataMessage = health.lastError
        || `跳过 ${result.snapshot.skippedRecords} 条损坏记录`
    } else if (health.lastError) {
      state.inputDataStatus = 'degraded'
      state.inputDataMessage = health.lastError
    } else if (result.snapshot.buckets.length) {
      state.inputDataStatus = 'ready'
      state.inputDataMessage = 'iTime 键盘字符键计数已连接'
    } else {
      state.inputDataStatus = 'empty'
      state.inputDataMessage = '已启动，等待记录'
    }
    state.lastDataRefreshAt = Date.now()
  } catch (error) {
    if (request !== inputRequest) return
    state.inputDataStatus = 'error'
    state.inputDataMessage = errorMessage(error, '输入数据不可用')
  }
}

async function refreshActivityData(): Promise<void> {
  if (!desktopRuntime) return
  const request = ++activityRequest
  state.activityDataStatus = 'loading'
  state.activityDataMessage = '读取中'
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  // 14 days (well under the 32d backend cap): the rolling week plus the
  // previous-week baseline used by 周报 comparison.
  startDate.setDate(startDate.getDate() - 14)
  try {
    const result = await loadActivityData({ start: startDate.getTime(), end: selectedEnd })
    if (request !== activityRequest) return
    liveActivityDataset.value = result.dataset
    activityDates.value = [...new Set([
      ...activityDates.value,
      ...result.dataset.events.map((event) => localDate(new Date(event.start))),
    ])].sort()
    updateAvailableDates()
    if (!result.snapshot.health.collectorRunning) {
      state.activityDataStatus = 'error'
      state.activityDataMessage = '采集器未运行；请尝试重启应用'
    } else if (result.snapshot.health.lastError) {
      state.activityDataStatus = 'degraded'
      state.activityDataMessage = `写入异常：${result.snapshot.health.lastError}`
    } else if (result.snapshot.skippedRecords > 0) {
      state.activityDataStatus = 'degraded'
      state.activityDataMessage = `跳过 ${result.snapshot.skippedRecords} 条损坏记录`
    } else if (result.snapshot.intervals.length) {
      state.activityDataStatus = 'ready'
      state.activityDataMessage = '已连接'
    } else {
      state.activityDataStatus = 'empty'
      state.activityDataMessage = '已开始记录'
    }
    state.lastDataRefreshAt = Date.now()
  } catch (error) {
    if (request !== activityRequest) return
    state.activityDataStatus = 'error'
    state.activityDataMessage = errorMessage(error, '活动记录不可用')
  }
}

async function refreshProviderData(): Promise<void> {
  if (!desktopRuntime) return
  if (!state.providerConsent.aiAgentToolsEnabled) {
    liveProviderDataset.value = { version: 'itime-local-provider-v1', events: [] }
    providerDates.value = []
    updateAvailableDates()
    state.providerDataStatus = 'disabled'
    state.providerDataMessage = '未授权；不扫描'
    return
  }
  const request = ++providerRequest
  state.providerDataStatus = 'loading'
  state.providerDataMessage = '读取中'
  const selectedEnd = dayRange(state.selectedDate).end
  const startDate = new Date(selectedEnd)
  startDate.setDate(startDate.getDate() - 7)
  try {
    const result = await loadProviderActivityData({ start: startDate.getTime(), end: selectedEnd })
    if (request !== providerRequest) return
    liveProviderDataset.value = result.dataset
    providerDates.value = [...new Set(result.dataset.events.map((event) => localDate(new Date(event.start))))].sort()
    updateAvailableDates()
    state.providerConsent = result.snapshot.consent
    const installedTools = result.snapshot.capabilities.tools.filter((tool) => tool.installed)
    const readyTools = installedTools.filter((tool) => tool.exactDuration)
    if (result.snapshot.status === 'disabled') {
      state.providerDataStatus = 'disabled'
      state.providerDataMessage = '未授权；不扫描'
    } else if (result.snapshot.status === 'unavailable') {
      // Uninstalled catalog entries are silent — never list them as "不可用".
      state.providerDataStatus = 'empty'
      if (result.snapshot.diagnostics.permissionFailures > 0) {
        state.providerDataStatus = 'error'
        state.providerDataMessage = '无读取权限；请在系统中检查目录权限'
      } else if (result.snapshot.diagnostics.readFailures > 0) {
        state.providerDataStatus = 'error'
        state.providerDataMessage = '目录读取失败；请稍后重试或重启应用'
      } else {
        state.providerDataMessage = '未检测到支持的 AI 编码工具'
      }
    } else if (result.snapshot.status === 'partial') {
      const { diagnostics } = result.snapshot
      state.providerDataStatus = 'degraded'
      if (diagnostics.permissionFailures > 0) {
        state.providerDataMessage = '部分可用；部分目录无权限'
      } else if (diagnostics.readFailures > 0) {
        state.providerDataMessage = '部分可用；部分目录读取失败'
      } else if (diagnostics.badLines + diagnostics.badEvents > 0) {
        state.providerDataMessage = '部分可用；已忽略异常记录'
      } else if (readyTools.length) {
        state.providerDataMessage = `${readyTools.length} 个工具可用`
      } else {
        // Installed but no exact parser yet — not "broken", just not timed.
        state.providerDataMessage = installedTools.length
          ? '已安装；会话计时待接入'
          : '已安装；会话计时待接入'
      }
    } else if (result.snapshot.intervals.length) {
      state.providerDataStatus = 'ready'
      state.providerDataMessage = `${result.snapshot.intervals.length} 个执行区间`
    } else {
      state.providerDataStatus = 'empty'
      state.providerDataMessage = '已连接；当日无执行区间'
    }
    state.lastDataRefreshAt = Date.now()
  } catch (error) {
    if (request !== providerRequest) return
    state.providerDataStatus = 'error'
    state.providerDataMessage = errorMessage(error, '会话不可用')
  }
}

async function syncProviderConsent(): Promise<void> {
  if (!desktopRuntime) return
  state.providerConsentStatus = 'loading'
  try {
    state.providerConsent = await loadProviderConsent()
    state.providerConsentStatus = 'ready'
    if (state.providerConsent.aiAgentToolsEnabled) {
      state.providerDataStatus = 'loading'
      state.providerDataMessage = '读取中'
    } else {
      state.providerDataStatus = 'disabled'
      state.providerDataMessage = '未授权；不扫描'
    }
  } catch (error) {
    state.providerConsentStatus = 'error'
    state.providerDataStatus = 'error'
    state.providerDataMessage = errorMessage(error, '授权读取失败')
  }
}

async function updateProviderConsent(update: Partial<Pick<ProviderConsent, 'noticeSeen' | 'aiAgentToolsEnabled'>>): Promise<void> {
  if (!desktopRuntime) return
  state.providerConsentStatus = 'loading'
  try {
    const consent = await saveProviderConsent({
      ...state.providerConsent,
      ...update,
    })
    state.providerConsent = consent
    state.providerConsentStatus = 'ready'
    await refreshProviderData()
  } catch (error) {
    state.providerConsentStatus = 'error'
    showToast(errorMessage(error, '授权保存失败'), 'error')
  }
}

async function refreshAutostart(): Promise<void> {
  if (!desktopRuntime) return
  state.autostartStatus = 'loading'
  try {
    state.autostartEnabled = await getAutostartEnabled()
    state.autostartStatus = 'ready'
    state.autostartMessage = state.autostartEnabled ? '已注册开机启动' : '未随系统启动'
  } catch (error) {
    state.autostartStatus = 'error'
    state.autostartMessage = errorMessage(error, '读取失败')
  }
}

async function setAutostart(enabled: boolean): Promise<void> {
  if (!desktopRuntime || state.autostartStatus === 'loading') return
  state.autostartStatus = 'loading'
  try {
    const confirmed = await setDesktopAutostart(enabled)
    state.autostartEnabled = confirmed
    state.autostartStatus = confirmed === enabled ? 'ready' : 'error'
    state.autostartMessage = confirmed === enabled
      ? (confirmed ? '已开启' : '已关闭')
      : '状态不一致'
  } catch (error) {
    state.autostartStatus = 'error'
    state.autostartMessage = errorMessage(error, '修改失败')
  }
}

// 错误类 toast 停留更久并带 tone 供视图区分；信息类维持短提示。
function showToast(message: string, tone: 'default' | 'error' = 'default'): void {
  const request = ++toastRequest
  state.toast = message
  state.toastTone = tone
  window.setTimeout(() => {
    if (request === toastRequest) state.toast = ''
  }, tone === 'error' ? 6000 : 2600)
}

function applyLocalDataStatus(status: LocalDataStatus): void {
  state.localData = status
  state.localDataStatus = status.health
  state.localDataMessage = status.message
  // `startAt` is the authoritative earliest-record bound — retention changes
  // and clear-all must immediately shrink the navigable date range.
  updateAvailableDates()
}

async function refreshLocalData(): Promise<void> {
  if (!desktopRuntime) return
  const request = ++localDataRequest
  state.localDataStatus = 'loading'
  state.localDataMessage = '检查中'
  try {
    const status = await loadLocalDataStatus()
    if (request !== localDataRequest) return
    applyLocalDataStatus(status)
  } catch (error) {
    if (request !== localDataRequest) return
    state.localDataStatus = 'error'
    state.localDataMessage = errorMessage(error, '读取失败')
  }
}

async function openLocalData(): Promise<void> {
  if (!desktopRuntime || state.localDataBusy) return
  state.localDataBusy = 'open'
  try {
    await openLocalDataDirectory()
  } catch (error) {
    showToast(errorMessage(error, '无法打开目录'), 'error')
  } finally {
    state.localDataBusy = null
  }
}

async function updateDataRetention(retentionDays: DataRetentionDays): Promise<void> {
  if (!desktopRuntime || state.localDataBusy) return
  state.localDataBusy = 'retention'
  state.localDataStatus = 'loading'
  state.localDataMessage = '保存中'
  try {
    applyLocalDataStatus(await saveDataRetention(retentionDays))
    await Promise.all([refreshInputData(), refreshActivityData()])
    showToast(retentionDays === null ? '永久保留' : `保留 ${retentionDays} 天`)
  } catch (error) {
    state.localDataStatus = 'error'
    state.localDataMessage = errorMessage(error, '保留期更新失败')
    showToast(state.localDataMessage, 'error')
  } finally {
    state.localDataBusy = null
  }
}

async function exportLocalRecords(format: DataExportFormat): Promise<void> {
  if (!desktopRuntime || state.localDataBusy) return
  state.localDataBusy = format
  state.localDataExportMessage = `导出 ${format.toUpperCase()}…`
  try {
    const result = await exportLocalData(format)
    state.localDataExportMessage = `已导出 ${result.activityRecords + result.keyboardRecords} 条：${result.path}`
    await refreshLocalData()
    showToast(`${format.toUpperCase()} 已导出`)
  } catch (error) {
    state.localDataExportMessage = errorMessage(error, `导出 ${format.toUpperCase()} 失败`)
    showToast(state.localDataExportMessage, 'error')
  } finally {
    state.localDataBusy = null
  }
}

async function clearLocalRecords(): Promise<boolean> {
  if (!desktopRuntime || state.localDataBusy) return false
  state.localDataBusy = 'clear'
  state.localDataStatus = 'loading'
  state.localDataMessage = '删除中'
  try {
    const status = await clearAllLocalData()
    liveActivityDataset.value = { version: 'itime-local-activity-v1', events: [] }
    liveKeyboardDataset.value = { version: 'itime-keyboard-v1', events: [] }
    liveProviderDataset.value = { version: 'itime-local-provider-v1', events: [] }
    providerDates.value = []
    liveInputProvider.value = null
    inputDates.value = []
    activityDates.value = []
    // The backend revokes AI-tool consent as part of delete-all; mirror the
    // default locally so the AI page stops rendering stale data and a future
    // enable must pass the notice gate again.
    state.providerConsent = { ...defaultProviderConsent }
    updateAvailableDates()
    applyLocalDataStatus(status)
    await Promise.all([refreshInputData(), refreshActivityData(), refreshProviderData()])
    showToast('本地记录已删除')
    return true
  } catch (error) {
    state.localDataStatus = 'error'
    state.localDataMessage = errorMessage(error, '删除失败')
    showToast(state.localDataMessage, 'error')
    return false
  } finally {
    state.localDataBusy = null
  }
}

function receiveReminder(occurrence: ReminderOccurrence): boolean {
  if (state.dismissedReminderOccurrences.includes(occurrence.occurrenceId)) return false
  state.currentReminder = occurrence
  return true
}

function dismissCurrentReminder(): void {
  const occurrence = state.currentReminder
  if (!occurrence) return
  state.dismissedReminderOccurrences = [
    ...new Set([...state.dismissedReminderOccurrences, occurrence.occurrenceId]),
  ].slice(-90)
  state.currentReminder = null
  showToast('已关闭本次提示；下个休息间隔仍会提醒')
}

export function useAppStore() {
  return {
    state,
    themeRevision,
    day,
    week,
    previousWeek,
    isToday,
    input,
    inputHistory,
    selectedTool,
    stepDate,
    goToToday,
    selectDate,
    openTool,
    closeTool,
    setRecording,
    syncRecording,
    applyTheme,
    refreshInputData,
    refreshActivityData,
    refreshProviderData,
    syncProviderConsent,
    updateProviderConsent,
    refreshAutostart,
    setAutostart,
    refreshLocalData,
    openLocalData,
    updateDataRetention,
    exportLocalRecords,
    clearLocalRecords,
    showToast,
    receiveReminder,
    dismissCurrentReminder,
  }
}

const stopThemeObserver = observeSystemTheme(
  () => state.theme,
  (theme) => {
    applyDocumentTheme(previewTheme ?? theme)
    themeRevision.value += 1
  },
)

let desktopTeardown: (() => void) | null = null

if (desktopRuntime) {
  const dataRefreshInterval = 60_000
  const providerRefreshEveryCycles = 5
  let refreshTimer: number | null = null
  let refreshCycle = 0
  let refreshPromise: Promise<void> | null = null
  let refreshQueued = false
  let providerRefreshQueued = false

  function clearRefreshTimer(): void {
    if (refreshTimer === null) return
    window.clearTimeout(refreshTimer)
    refreshTimer = null
  }

  function refreshRuntimeData(includeProvider: boolean): Promise<void> {
    refreshQueued = true
    providerRefreshQueued ||= includeProvider
    if (refreshPromise) return refreshPromise

    refreshPromise = (async () => {
      while (refreshQueued && document.visibilityState !== 'hidden') {
        const shouldRefreshProvider = providerRefreshQueued
        refreshQueued = false
        providerRefreshQueued = false
        await refreshInputData()
        await refreshActivityData()
        if (shouldRefreshProvider) await refreshProviderData()
      }
    })().finally(() => {
      refreshPromise = null
    })
    return refreshPromise
  }

  function scheduleRefresh(): void {
    clearRefreshTimer()
    if (document.visibilityState === 'hidden') return
    refreshTimer = window.setTimeout(async () => {
      refreshCycle += 1
      await refreshRuntimeData(refreshCycle % providerRefreshEveryCycles === 0)
      scheduleRefresh()
    }, dataRefreshInterval)
  }

  function refreshNow(includeProvider = true): void {
    clearRefreshTimer()
    void refreshRuntimeData(includeProvider).finally(scheduleRefresh)
  }

  function handleVisibilityChange(): void {
    if (document.visibilityState === 'hidden') {
      clearRefreshTimer()
      return
    }
    refreshNow()
  }

  void (async () => {
    await syncProviderConsent()
    await refreshLocalData()
    refreshNow(state.providerConsentStatus === 'ready')
    await refreshAutostart()
  })()
  const stopSelectedDateWatch = watch(() => state.selectedDate, () => refreshNow())
  document.addEventListener('visibilitychange', handleVisibilityChange)
  desktopTeardown = () => {
    clearRefreshTimer()
    stopSelectedDateWatch()
    document.removeEventListener('visibilitychange', handleVisibilityChange)
  }
}

// Module-level timers/listeners/watchers have no component lifecycle; release
// them on Vite HMR dispose so a hot-reloaded store never double-registers.
if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    stopPersistWatch()
    stopThemeWatch()
    stopThemeObserver()
    desktopTeardown?.()
    desktopTeardown = null
  })
}
