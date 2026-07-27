import { isTauriRuntime } from './desktop'

export async function queryAutostartState(
  isEnabled: () => Promise<boolean>,
): Promise<boolean> {
  return isEnabled()
}

export async function changeAutostartState(
  enabled: boolean,
  api: {
    enable: () => Promise<void>
    disable: () => Promise<void>
    isEnabled: () => Promise<boolean>
  },
): Promise<boolean> {
  if (enabled) await api.enable()
  else await api.disable()
  return api.isEnabled()
}

export async function getAutostartEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { isEnabled } = await import('@tauri-apps/plugin-autostart')
  return queryAutostartState(isEnabled)
}

export async function setDesktopAutostart(enabled: boolean): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { disable, enable, isEnabled } = await import('@tauri-apps/plugin-autostart')
  return changeAutostartState(enabled, { disable, enable, isEnabled })
}
