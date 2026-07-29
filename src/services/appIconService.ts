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
const ICON_RESOLVER_VERSION = 3
const MAX_MEMORY_ENTRIES = 256
const iconSources: ReadonlySet<string> = new Set([
  'cache', 'shell_item', 'sh_get_file_info', 'extract_icon', 'package_asset', 'shortcut', 'fallback', 'embedded',
])
const memory = new Map<string, IconResolveResult>()
const inflight = new Map<string, Promise<IconResolveResult>>()
const listeners = new Set<Listener>()
let eventBound = false
let bridgePromise: Promise<void> | null = null

function clampSize(size: number | undefined): number {
  return Math.min(256, Math.max(16, Math.round(size ?? DEFAULT_SIZE)))
}

function iconRequestKey(identity: string, size: number): string {
  return `${identity}\u0000${clampSize(size)}\u0000${ICON_RESOLVER_VERSION}`
}

function identityKeyPrefix(identity: string): string {
  return `${identity}\u0000`
}

function remember(result: IconResolveResult): void {
  const key = iconRequestKey(result.appIdentity, result.width)
  memory.delete(key)
  memory.set(key, result)
  while (memory.size > MAX_MEMORY_ENTRIES) {
    const oldest = memory.keys().next().value
    if (typeof oldest !== 'string') break
    memory.delete(oldest)
  }
}

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
  const width = clampSize(response.width)
  return {
    appIdentity: response.appIdentity,
    status,
    cachePath: response.cachePath,
    iconUrl: status === 'resolved' ? await toUrl(response.cachePath) : null,
    iconSource: iconSources.has(response.iconSource) ? response.iconSource as IconSource : 'fallback',
    width,
    height: clampSize(response.height || width),
    errorCode: response.errorCode,
  }
}

async function ensureEventBridge(): Promise<void> {
  if (eventBound || !isTauriRuntime()) return
  if (!bridgePromise) {
    bridgePromise = listenDesktop<NativeResponse>('app-icon-updated', (payload) => {
      void normalizeNative(payload).then((result) => {
        remember(result)
        inflight.delete(iconRequestKey(result.appIdentity, result.width))
        for (const listener of listeners) listener(result)
      })
    }).then(() => {
      eventBound = true
    }).catch((error) => {
      bridgePromise = null
      throw error
    })
  }
  await bridgePromise
}

export function subscribeAppIcons(listener: Listener): () => void {
  listeners.add(listener)
  void ensureEventBridge()
  return () => {
    listeners.delete(listener)
  }
}

export function peekAppIcon(identity: string, requestedSize = DEFAULT_SIZE): IconResolveResult | undefined {
  return memory.get(iconRequestKey(identity, requestedSize))
}

export function forgetAppIcon(identity: string): void {
  const prefix = identityKeyPrefix(identity)
  for (const key of memory.keys()) {
    if (key.startsWith(prefix)) memory.delete(key)
  }
  for (const key of inflight.keys()) {
    if (key.startsWith(prefix)) inflight.delete(key)
  }
}

export async function resolveAppIcon(
  input: AppIdentityInput & { requestedSize?: number },
): Promise<IconResolveResult> {
  const { identity } = buildAppIdentity(input)
  const requestedSize = clampSize(input.requestedSize)
  const key = iconRequestKey(identity, requestedSize)
  const cached = memory.get(key)
  if (cached?.status === 'resolved' && cached.iconUrl) return cached

  const pending = inflight.get(key)
  if (pending) return pending

  if (!isTauriRuntime()) {
    const offline: IconResolveResult = {
      appIdentity: identity,
      status: 'unknown',
      iconUrl: null,
      iconSource: 'fallback',
      width: requestedSize,
      height: requestedSize,
    }
    remember(offline)
    return offline
  }

  const task = (async () => {
    try {
      await ensureEventBridge()
      const { invoke } = await import('@tauri-apps/api/core')
      const response = await invoke<NativeResponse>('resolve_app_icon', {
        request: {
          appIdentity: input.appIdentity ?? input.iconKey ?? null,
          executablePath: input.executablePath ?? null,
          aumid: input.aumid ?? null,
          packageFullName: input.packageFullName ?? null,
          packageFamilyName: input.packageFamilyName ?? null,
          siteHost: input.siteHost ?? null,
          processId: input.processId ?? null,
          requestedSize,
        },
      })
      const result = await normalizeNative(response)
      remember(result)
      for (const listener of listeners) listener(result)
      return result
    } catch (error) {
      const failed: IconResolveResult = {
        appIdentity: identity,
        status: 'failed',
        iconUrl: null,
        iconSource: 'fallback',
        width: requestedSize,
        height: requestedSize,
        errorCode: error instanceof Error ? error.message : String(error),
      }
      remember(failed)
      for (const listener of listeners) listener(failed)
      return failed
    } finally {
      inflight.delete(key)
    }
  })()

  inflight.set(key, task)
  return task
}

export function clearAppIconMemory(): void {
  memory.clear()
  inflight.clear()
}
