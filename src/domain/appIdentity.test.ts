import { describe, expect, it } from 'vitest'
import { buildAppIdentity, canonicalAppKey, identityAccent, identityGlyph, normalizeLogicalKey } from './appIdentity'

describe('appIdentity', () => {
  it('prefers aumid package identity', () => {
    const result = buildAppIdentity({
      appIdentity: 'chrome',
      executablePath: 'C:\\Apps\\chrome.exe',
      aumid: 'Chrome.App',
    })
    expect(result).toEqual({ identity: 'aumid:Chrome.App', kind: 'package' })
  })

  it('builds site identity with browser host', () => {
    const result = buildAppIdentity({
      iconKey: 'chrome',
      siteHost: 'GitHub.com',
    })
    expect(result.kind).toBe('browser_site')
    expect(result.identity).toBe('site:github.com@chrome')
  })

  it('does not expose an executable path in the public identity', () => {
    const result = buildAppIdentity({ executablePath: 'C:\\Users\\person\\Apps\\Secret\\tool.exe' })
    expect(result.kind).toBe('executable_path')
    expect(result.identity).toMatch(/^exe:[0-9a-f]{16}$/)
    expect(result.identity).not.toContain('users')
    expect(result.identity).not.toContain('secret')
  })

  it('normalizes logical keys and glyphs', () => {
    expect(normalizeLogicalKey('VS Code')).toBe('vs-code')
    expect(identityGlyph('VS Code')).toBe('V')
    expect(identityGlyph('微信')).toBe('微')
    expect(identityGlyph(undefined, 'app:explorer')).toBe('E')
  })

  it('maps display names to stable icon identities', () => {
    expect(canonicalAppKey('文件资源管理器')).toBe('explorer')
    expect(canonicalAppKey('Claude Code')).toBe('claude')
    expect(canonicalAppKey('Codex')).toBe('codex')
    expect(canonicalAppKey('WindowsTerminal')).toBe('windows-terminal')
    expect(canonicalAppKey('clash-verge')).toBe('clash-verge')
    expect(canonicalAppKey('Weixin')).toBe('wechat')
  })

  it('returns stable accent colors for the same identity', () => {
    document.documentElement.dataset.theme = 'light'
    const a = identityAccent('app:vscode')
    const b = identityAccent('app:vscode')
    expect(a).toEqual(b)
    expect(a.background).toMatch(/^hsl\(/)
  })

  it('uses a darker glyph palette under dark theme', () => {
    document.documentElement.dataset.theme = 'light'
    const light = identityAccent('app:vscode')
    document.documentElement.dataset.theme = 'dark'
    const dark = identityAccent('app:vscode')
    expect(dark).not.toEqual(light)
    expect(dark.background).toMatch(/^hsl\(/)
    document.documentElement.dataset.theme = 'light'
  })
})
