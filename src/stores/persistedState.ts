import type { ClosePreference, ThemeMode } from './appStore'

export interface PersistedState {
  theme: ThemeMode
  recording: boolean
  reminders: boolean
  hideToTray: boolean
  closePreference: ClosePreference
  heatmapEnabled: boolean
  shortcutsEnabled: boolean
  aiNotifications: boolean
  quietStart: string
  quietEnd: string
  goals: Record<string, number>
  migrationState: 'notFound' | 'partial' | 'ready' | 'imported'
  deletedInputDates: string[]
}

export const persistedDefaults: PersistedState = {
  theme: 'light',
  recording: true,
  reminders: true,
  hideToTray: true,
  closePreference: 'ask',
  heatmapEnabled: true,
  shortcutsEnabled: true,
  aiNotifications: true,
  quietStart: '22:00',
  quietEnd: '08:00',
  goals: { learning: 120, development: 180, ai: 180, continuous: 50 },
  migrationState: 'partial',
  deletedInputDates: [],
}

export function loadPersistedState(): PersistedState {
  if (typeof localStorage === 'undefined') return persistedDefaults
  try {
    const value = JSON.parse(localStorage.getItem('itime-prototype-state') ?? '{}')
    return {
      ...persistedDefaults,
      ...value,
      goals: { ...persistedDefaults.goals, ...(value.goals ?? {}) },
    }
  } catch {
    return persistedDefaults
  }
}

export function savePersistedState(value: PersistedState): void {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem('itime-prototype-state', JSON.stringify(value))
}
