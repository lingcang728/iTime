import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const html = readFileSync(resolve(process.cwd(), 'index.html'), 'utf8')
const bootstrap = html.match(/<script>\s*([\s\S]*?)\s*<\/script>/)?.[1] ?? ''

function runBootstrap(search: string, systemDark: boolean): void {
  window.history.replaceState({}, '', `/${search}`)
  vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: systemDark })))
  window.eval(bootstrap)
}

describe('pre-render theme bootstrap', () => {
  beforeEach(() => {
    localStorage.clear()
    document.head.innerHTML = '<meta name="theme-color" content="#f7f7f4">'
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.style.removeProperty('color-scheme')
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    window.history.replaceState({}, '', '/')
  })

  it('runs before the application module is requested', () => {
    expect(bootstrap.length).toBeGreaterThan(0)
    expect(html.indexOf('<script>')).toBeLessThan(html.indexOf('<script type="module"'))
  })

  it('applies a saved explicit theme before rendering', () => {
    localStorage.setItem('itime-prototype-state', JSON.stringify({ theme: 'dark' }))
    runBootstrap('', false)
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.style.colorScheme).toBe('dark')
  })

  it('uses the system for new profiles and keeps preview overrides temporary', () => {
    runBootstrap('', true)
    expect(document.documentElement.dataset.theme).toBe('dark')

    localStorage.setItem('itime-prototype-state', JSON.stringify({ theme: 'dark' }))
    runBootstrap('?theme=light', true)
    expect(document.documentElement.dataset.theme).toBe('light')
    expect(JSON.parse(localStorage.getItem('itime-prototype-state') ?? '{}').theme).toBe('dark')
  })
})
