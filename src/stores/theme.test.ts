import { afterEach, describe, expect, it, vi } from 'vitest'
import { applyDocumentTheme, observeSystemTheme, resolveTheme, systemPrefersDark } from './theme'

describe('theme resolution', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.style.removeProperty('color-scheme')
  })

  it('resolves explicit themes without consulting the system value', () => {
    expect(resolveTheme('light', true)).toBe('light')
    expect(resolveTheme('dark', false)).toBe('dark')
  })

  it('resolves system mode from the media preference', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
  })

  it('reads and applies the system theme before the app renders', () => {
    vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: true })))
    expect(systemPrefersDark()).toBe(true)
    applyDocumentTheme('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.style.colorScheme).toBe('dark')
  })

  it('tracks system changes only while the saved preference follows the system', () => {
    let listener: ((event: { matches: boolean }) => void) | undefined
    let removed = false
    vi.stubGlobal('matchMedia', vi.fn(() => ({
      matches: false,
      addEventListener: (_type: string, callback: (event: { matches: boolean }) => void) => { listener = callback },
      removeEventListener: () => { removed = true },
    })))
    let savedMode: 'light' | 'dark' | 'system' = 'system'
    const applied: string[] = []
    const stop = observeSystemTheme(() => savedMode, (theme) => applied.push(theme))

    listener?.({ matches: true })
    savedMode = 'light'
    listener?.({ matches: false })

    expect(applied).toEqual(['dark'])
    stop()
    expect(removed).toBe(true)
  })
})
