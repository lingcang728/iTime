export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export async function minimizeWindow(): Promise<void> {
  if (!isTauriRuntime()) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().minimize()
}

export async function toggleMaximizeWindow(): Promise<void> {
  if (!isTauriRuntime()) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().toggleMaximize()
}

export async function isWindowMaximized(): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  return getCurrentWindow().isMaximized()
}

export async function startWindowDragging(): Promise<void> {
  if (!isTauriRuntime()) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  const currentWindow = getCurrentWindow()
  if (await currentWindow.isMaximized()) await currentWindow.unmaximize()
  await currentWindow.startDragging()
}

export async function listenWindowResize(listener: () => void): Promise<() => void> {
  if (!isTauriRuntime()) return () => undefined
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  return getCurrentWindow().onResized(listener)
}

export async function hideWindow(): Promise<void> {
  if (!isTauriRuntime()) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().hide()
}

export async function quitApplication(): Promise<void> {
  if (!isTauriRuntime()) return
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('quit_app')
}

export async function setDesktopRecording(recording: boolean): Promise<void> {
  if (!isTauriRuntime()) return
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('set_recording_state', { recording })
}

export async function getDesktopRecording(): Promise<boolean> {
  if (!isTauriRuntime()) return true
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<boolean>('get_recording_state')
}

export async function configureDesktopReminders(input: {
  enabled: boolean
  intervalMinutes: number
  quietStart: string
  quietEnd: string
}): Promise<void> {
  if (!isTauriRuntime()) return
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('configure_reminders', {
    enabled: input.enabled,
    intervalMinutes: input.intervalMinutes,
    quietStart: input.quietStart,
    quietEnd: input.quietEnd,
  })
}

export async function listenDesktop<T>(event: string, listener: (payload: T) => void): Promise<() => void> {
  if (!isTauriRuntime()) return () => undefined
  const { listen } = await import('@tauri-apps/api/event')
  return listen<T>(event, ({ payload }) => listener(payload))
}
