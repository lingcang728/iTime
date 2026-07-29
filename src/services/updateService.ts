import { reactive } from 'vue'
import { isTauriRuntime } from '../platform/desktop'

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'installing'
  | 'failed'
  | 'upToDate'

export interface UpdateViewState {
  status: UpdateStatus
  currentVersion: string
  version: string
  date: string
  notes: string
  sizeBytes: number | null
  downloadedBytes: number
  totalBytes: number | null
  error: string
}

const AUTO_CHECK_KEY = 'itime-updater-last-auto-check-v2'
const AUTO_CHECK_INTERVAL = 24 * 60 * 60 * 1_000

export const updateState = reactive<UpdateViewState>({
  status: 'idle',
  currentVersion: '',
  version: '',
  date: '',
  notes: '',
  sizeBytes: null,
  downloadedBytes: 0,
  totalBytes: null,
  error: '',
})

type TauriUpdate = Awaited<ReturnType<typeof import('@tauri-apps/plugin-updater')['check']>>
let pendingUpdate: TauriUpdate = null

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function positiveNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : null
}

function packageSize(raw: Record<string, unknown>): number | null {
  const direct = positiveNumber(raw.size)
  if (direct !== null) return direct
  const platforms = raw.platforms
  if (!platforms || typeof platforms !== 'object') return null
  for (const value of Object.values(platforms)) {
    if (value && typeof value === 'object') {
      const size = positiveNumber((value as Record<string, unknown>).size)
      if (size !== null) return size
    }
  }
  return null
}

async function loadCurrentVersion(): Promise<void> {
  if (updateState.currentVersion || !isTauriRuntime()) return
  const { getVersion } = await import('@tauri-apps/api/app')
  updateState.currentVersion = await getVersion()
}

export async function checkForDesktopUpdate(manual = false): Promise<void> {
  if (!isTauriRuntime()) return
  updateState.status = 'checking'
  updateState.error = ''
  try {
    await loadCurrentVersion()
    const lastCheck = Number(localStorage.getItem(AUTO_CHECK_KEY) ?? 0)
    if (!manual && Date.now() - lastCheck < AUTO_CHECK_INTERVAL) {
      updateState.status = 'upToDate'
      return
    }

    const { check } = await import('@tauri-apps/plugin-updater')
    pendingUpdate = await check({ timeout: 15_000 })
    if (!pendingUpdate) {
      localStorage.setItem(AUTO_CHECK_KEY, String(Date.now()))
      updateState.status = 'upToDate'
      updateState.version = ''
      updateState.notes = ''
      updateState.sizeBytes = null
      return
    }
    localStorage.removeItem(AUTO_CHECK_KEY)
    updateState.status = 'available'
    updateState.currentVersion = pendingUpdate.currentVersion
    updateState.version = pendingUpdate.version
    updateState.date = pendingUpdate.date ?? ''
    updateState.notes = pendingUpdate.body ?? ''
    updateState.sizeBytes = packageSize(pendingUpdate.rawJson)
    updateState.downloadedBytes = 0
    updateState.totalBytes = updateState.sizeBytes
  } catch (error) {
    pendingUpdate = null
    updateState.status = 'failed'
    updateState.error = errorMessage(error)
  }
}

export async function downloadAndInstallDesktopUpdate(): Promise<void> {
  if (!pendingUpdate || updateState.status !== 'available') {
    updateState.status = 'failed'
    updateState.error = '没有可安装的更新，请重新检查。'
    return
  }

  const { invoke } = await import('@tauri-apps/api/core')
  let prepared = false
  let portable = false
  try {
    const preparation = await invoke<{ portable: boolean }>('prepare_for_update')
    portable = preparation.portable
    prepared = true
    updateState.status = 'downloading'
    updateState.error = ''
    updateState.downloadedBytes = 0
    await pendingUpdate.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        updateState.totalBytes = event.data.contentLength ?? updateState.sizeBytes
      } else if (event.event === 'Progress') {
        updateState.downloadedBytes += event.data.chunkLength
      } else {
        updateState.status = 'installing'
      }
    })
    updateState.status = 'installing'
    if (portable) {
      await invoke('launch_migrated_install')
    } else {
      const { relaunch } = await import('@tauri-apps/plugin-process')
      await relaunch()
    }
  } catch (error) {
    if (prepared) {
      try {
        await invoke('cancel_update_preparation')
      } catch (restoreError) {
        updateState.error = `${errorMessage(error)}；恢复采集失败：${errorMessage(restoreError)}`
        updateState.status = 'failed'
        return
      }
    }
    updateState.status = 'failed'
    updateState.error = errorMessage(error)
  }
}

export function resetUpdateServiceForTests(): void {
  pendingUpdate = null
  Object.assign(updateState, {
    status: 'idle',
    currentVersion: '',
    version: '',
    date: '',
    notes: '',
    sizeBytes: null,
    downloadedBytes: 0,
    totalBytes: null,
    error: '',
  } satisfies UpdateViewState)
  localStorage.removeItem(AUTO_CHECK_KEY)
}
