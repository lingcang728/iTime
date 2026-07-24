/** Stable application identity helpers shared by UI and native icon resolution. */

export type AppIdentityKind = 'executable_path' | 'package' | 'browser_site' | 'logical'

export interface AppIdentityInput {
  appIdentity?: string | null
  iconKey?: string | null
  executablePath?: string | null
  aumid?: string | null
  packageFullName?: string | null
  packageFamilyName?: string | null
  siteHost?: string | null
  appName?: string | null
  processId?: number | null
}

const opaqueIdentityPattern = /^(?:process|exe):/i

const appNameAliases: Record<string, string> = {
  chrome: 'chrome',
  'googlechrome': 'chrome',
  code: 'vscode',
  vscode: 'vscode',
  'visualstudiocode': 'vscode',
  codex: 'codex',
  chatgpt: 'chatgpt',
  claude: 'claude',
  claudecode: 'claude',
  typeless: 'typeless',
  antigravity: 'antigravity',
  explorer: 'explorer',
  fileexplorer: 'explorer',
  '文件资源管理器': 'explorer',
  microsoftedge: 'msedge',
  edge: 'msedge',
  wechat: 'wechat',
  weixin: 'wechat',
  微信: 'wechat',
  windowsterminal: 'windows-terminal',
  terminal: 'windows-terminal',
  clashverge: 'clash-verge',
  clash: 'clash-verge',
  itime: 'itime',
}

export function canonicalAppKey(value?: string | null): string | null {
  const compact = value?.trim().toLocaleLowerCase().replace(/[\s._-]+/g, '')
  return compact ? appNameAliases[compact] ?? null : null
}

/**
 * Select the identity sent to the native icon resolver.
 * Anonymous activity ids are stable for aggregation but cannot locate an icon,
 * so prefer the captured display name unless a concrete package/path identity exists.
 */
export function iconResolverIdentity(input: AppIdentityInput): string | null {
  const concreteIdentity = Boolean(
    input.executablePath
    || input.aumid
    || input.packageFullName
    || input.packageFamilyName
    || input.siteHost,
  )
  if (concreteIdentity) {
    return input.appIdentity ?? input.iconKey ?? input.appName ?? null
  }

  const canonical =
    canonicalAppKey(input.appName)
    ?? canonicalAppKey(input.appIdentity)
    ?? canonicalAppKey(input.iconKey)
  if (canonical) return canonical

  if (opaqueIdentityPattern.test(input.appIdentity ?? '')) {
    return input.appName ?? input.iconKey ?? input.appIdentity ?? null
  }
  return input.appIdentity ?? input.iconKey ?? input.appName ?? null
}

export function normalizeLogicalKey(value: string | null | undefined): string | null {
  if (!value) return null
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return normalized || null
}

function privatePathKey(value: string): string {
  const normalized = value.trim().toLowerCase().replace(/\//g, '\\')
  let hash = 0xcbf29ce484222325n
  for (const byte of new TextEncoder().encode(normalized)) {
    hash ^= BigInt(byte)
    hash = BigInt.asUintN(64, hash * 0x100000001b3n)
  }
  return hash.toString(16).padStart(16, '0')
}

/** Frontend mirror of Rust normalize_app_identity for cache keys / UI state. */
export function buildAppIdentity(input: AppIdentityInput): { identity: string; kind: AppIdentityKind } {
  const host = input.siteHost?.trim()
  if (host) {
    const browser =
      (input.executablePath?.trim() ? privatePathKey(input.executablePath) : null) ??
      normalizeLogicalKey(input.appIdentity) ??
      normalizeLogicalKey(input.iconKey) ??
      'browser'
    return { identity: `site:${host.toLowerCase()}@${browser}`, kind: 'browser_site' }
  }

  if (input.aumid?.trim()) {
    return { identity: `aumid:${input.aumid.trim()}`, kind: 'package' }
  }
  if (input.packageFullName?.trim()) {
    return { identity: `pkg:${input.packageFullName.trim()}`, kind: 'package' }
  }
  if (input.packageFamilyName?.trim()) {
    return { identity: `pkgfamily:${input.packageFamilyName.trim()}`, kind: 'package' }
  }

  const path = input.executablePath?.trim()
  if (path) {
    return { identity: `exe:${privatePathKey(path)}`, kind: 'executable_path' }
  }

  const logical =
    normalizeLogicalKey(input.appIdentity) ??
    normalizeLogicalKey(input.iconKey) ??
    normalizeLogicalKey(input.appName) ??
    'unknown'
  return { identity: `app:${logical}`, kind: 'logical' }
}

function isDarkTheme(): boolean {
  if (typeof document === 'undefined') return false
  return document.documentElement.dataset.theme === 'dark'
}

/** Stable pastel / dark-muted background from identity. */
export function identityAccent(identity: string): { background: string; color: string } {
  let hash = 0
  for (let i = 0; i < identity.length; i += 1) {
    hash = (hash * 31 + identity.charCodeAt(i)) >>> 0
  }
  const hues = [24, 38, 152, 168, 198, 222, 262, 286]
  const hue = hues[hash % hues.length]
  const dark = isDarkTheme()
  if (dark) {
    const sat = 18 + (hash % 10)
    const bgL = 18 + (hash % 7)
    const fgL = 74 + (hash % 10)
    return {
      background: `hsl(${hue} ${sat}% ${bgL}%)`,
      color: `hsl(${hue} ${Math.min(sat + 16, 40)}% ${fgL}%)`,
    }
  }
  const sat = 28 + (hash % 12)
  const light = 88 - (hash % 8)
  return {
    background: `hsl(${hue} ${sat}% ${light}%)`,
    color: `hsl(${hue} ${Math.min(sat + 18, 42)}% ${28 + (hash % 6)}%)`,
  }
}

export function identityGlyph(appName?: string | null, identity?: string | null): string {
  const name = appName?.trim()
  if (name) {
    const match = name.match(/[\p{Script=Han}\p{L}\p{N}]/u)
    if (match) return match[0]!.toLocaleUpperCase()
  }
  const logical = identity?.replace(/^app:/, '') ?? '?'
  const ch = logical.match(/[a-z0-9\u4e00-\u9fff]/i)?.[0]
  return (ch ?? '?').toLocaleUpperCase()
}
