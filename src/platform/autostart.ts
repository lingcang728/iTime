import { isTauriRuntime } from './desktop'

export async function getAutostartEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { isEnabled } = await import('@tauri-apps/plugin-autostart')
  return isEnabled()
}

export async function refreshDesktopAutostartRegistration(): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { disable, enable, isEnabled } = await import('@tauri-apps/plugin-autostart')
  if (!await isEnabled()) return false
  await disable()
  await enable()
  return isEnabled()
}

export async function setDesktopAutostart(enabled: boolean): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const { disable, enable, isEnabled } = await import('@tauri-apps/plugin-autostart')
  if (enabled) await enable()
  else await disable()
  return isEnabled()
}
