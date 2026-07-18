export type ThemeMode = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

export function resolveTheme(mode: ThemeMode, systemDark: boolean): ResolvedTheme {
  if (mode === 'system') return systemDark ? 'dark' : 'light'
  return mode
}

export function systemPrefersDark(): boolean {
  return typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches
}

export function applyDocumentTheme(theme: ResolvedTheme): void {
  document.documentElement.dataset.theme = theme
  document.documentElement.style.colorScheme = theme
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', theme === 'dark' ? '#151817' : '#f7f7f4')
}

export function observeSystemTheme(
  getMode: () => ThemeMode,
  onResolvedTheme: (theme: ResolvedTheme) => void,
): () => void {
  if (typeof matchMedia === 'undefined') return () => undefined
  const media = matchMedia('(prefers-color-scheme: dark)')
  const handleChange = (event: MediaQueryListEvent): void => {
    if (getMode() === 'system') onResolvedTheme(resolveTheme('system', event.matches))
  }
  media.addEventListener('change', handleChange)
  return () => media.removeEventListener('change', handleChange)
}
