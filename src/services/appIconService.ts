import { buildAppIdentity, type AppIdentityInput } from '../domain/appIdentity'
import { isTauriRuntime, listenDesktop } from '../platform/desktop'

export type IconStatus = 'loading' | 'resolved' | 'failed' | 'unknown'

export type IconSource =
  | 'cache'
  | 'shell_item'
  | 'sh_get_file_info'
  | 'extract_icon'
  | 'package_asset'
  | 'shortcut'
  | 'fallback'
  | 'embedded'

export interface IconResolveResult {
  appIdentity: string
  status: IconStatus
  cachePath?: string | null
  iconUrl?: string | null
  iconSource: IconSource
  width: number
  height: number
  errorCode?: string | null
}

type Listener = (result: IconResolveResult) => void

interface NativeResponse {
  appIdentity: string
  status: string
  cachePath?: string | null
  iconSource: string
  width: number
  height: number
  errorCode?: string | null
}

const DEFAULT_SIZE = 64
const memory = new Map<string, IconResolveResult>()
const inflight = new Map<string, Promise<IconResolveResult>>()
const listeners = new Set<Listener>()
let eventBound = false

async function toUrl(cachePath?: string | null): Promise<string | null> {
  if (!cachePath || !isTauriRuntime()) return null
  try {
    const { convertFileSrc } = await import('@tauri-apps/api/core')
    return convertFileSrc(cachePath)
  } catch {
    return null
  }
}

async function normalizeNative(response: NativeResponse): Promise<IconResolveResult> {
  const status = (['loading', 'resolved', 'failed', 'unknown'].includes(response.status)
    ? response.status
    : 'unknown') as IconStatus
  return {
    appIdentity: response.appIdentity,
    status,
    cachePath: response.cachePath,
    iconUrl: status === 'resolved' ? await toUrl(response.cachePath) : null,
    iconSource: (response.iconSource as IconSource) || 'fallback',
    width: response.width || DEFAULT_SIZE,
    height: response.height || DEFAULT_SIZE,
    errorCode: response.errorCode,
  }
}

async function ensureEventBridge(): Promise<void> {
  if (eventBound || !isTauriRuntime()) return
  eventBound = true
  await listenDesktop<NativeResponse>('app-icon-updated', (payload) => {
    void normalizeNative(payload).then((result) => {
      memory.set(result.appIdentity, result)
      inflight.delete(result.appIdentity)
      for (const listener of listeners) listener(result)
    })
  })
}

export function subscribeAppIcons(listener: Listener): () => void {
  listeners.add(listener)
  void ensureEventBridge()
  return () => {
    listeners.delete(listener)
  }
}

export function peekAppIcon(identity: string): IconResolveResult | undefined {
  return memory.get(identity)
}

export async function resolveAppIcon(
  input: AppIdentityInput & { requestedSize?: number },
): Promise<IconResolveResult> {
  const { identity } = buildAppIdentity(input)
  const cached = memory.get(identity)
  if (cached?.status === 'resolved' && cached.iconUrl) return cached
  if (cached?.status === 'failed') return cached

  const pending = inflight.get(identity)
  if (pending) return pending

  if (!isTauriRuntime()) {
    const offline: IconResolveResult = {
      appIdentity: identity,
      status: 'unknown',
      iconUrl: null,
      iconSource: 'fallback',
      width: input.requestedSize ?? DEFAULT_SIZE,
      height: input.requestedSize ?? DEFAULT_SIZE,
    }
    memory.set(identity, offline)
    return offline
  }

  await ensureEventBridge()

  const task = (async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const response = await invoke<NativeResponse>('resolve_app_icon', {
        request: {
          appIdentity: input.appIdentity ?? input.iconKey ?? null,
          executablePath: input.executablePath ?? null,
          aumid: input.aumid ?? null,
          packageFullName: input.packageFullName ?? null,
          packageFamilyName: input.packageFamilyName ?? null,
          siteHost: input.siteHost ?? null,
          requestedSize: input.requestedSize ?? DEFAULT_SIZE,
        },
      })
      const result = await normalizeNative(response)
      memory.set(result.appIdentity, result)
      for (const listener of listeners) listener(result)
      return result
    } catch (error) {
      const failed: IconResolveResult = {
        appIdentity: identity,
        status: 'failed',
        iconUrl: null,
        iconSource: 'fallback',
        width: input.requestedSize ?? DEFAULT_SIZE,
        height: input.requestedSize ?? DEFAULT_SIZE,
        errorCode: error instanceof Error ? error.message : String(error),
      }
      memory.set(identity, failed)
      for (const listener of listeners) listener(failed)
      return failed
    } finally {
      inflight.delete(identity)
    }
  })()

  inflight.set(identity, task)
  return task
}

export function clearAppIconMemory(): void {
  memory.clear()
  inflight.clear()
}
