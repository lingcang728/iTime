import { z } from 'zod'
import { goalDefinitions } from '../domain/goals'
import type { ClosePreference } from './appStore'
import type { ThemeMode } from './theme'

const STORAGE_KEY = 'itime-prototype-state'
const SCHEMA_VERSION = 4

export interface PersistedState {
  schemaVersion: typeof SCHEMA_VERSION
  theme: ThemeMode
  reminders: boolean
  closePreference: ClosePreference
  quietStart: string
  quietEnd: string
  goals: Record<string, number>
  dismissedReminderOccurrences: string[]
  rememberCloseChoice: boolean
}

export const persistedDefaults: PersistedState = {
  schemaVersion: SCHEMA_VERSION,
  theme: 'system',
  reminders: true,
  closePreference: 'ask',
  quietStart: '22:00',
  quietEnd: '08:00',
  goals: { learning: 120, development: 180, ai: 180, continuous: 50 },
  dismissedReminderOccurrences: [],
  rememberCloseChoice: false,
}

const storedSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  reminders: z.boolean().optional(),
  closePreference: z.enum(['ask', 'hide', 'quit']).optional(),
  quietStart: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/).optional(),
  quietEnd: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/).optional(),
  goals: z.record(z.string(), z.number().finite().nonnegative()).optional(),
  dismissedReminderOccurrences: z.array(z.string().min(1).max(200)).max(90).optional(),
  rememberCloseChoice: z.boolean().optional(),
}).strip()

export function loadPersistedState(): PersistedState {
  if (typeof localStorage === 'undefined') return { ...persistedDefaults }
  const raw = localStorage.getItem(STORAGE_KEY)
  try {
    const parsed = storedSchema.safeParse(JSON.parse(raw ?? '{}'))
    if (!parsed.success) {
      // Never silently drop user preferences: quarantine the corrupt blob so it
      // stays recoverable/debuggable instead of being overwritten on next save.
      if (raw) {
        try { localStorage.setItem(`${STORAGE_KEY}.corrupt`, raw) } catch { /* best effort */ }
      }
      return { ...persistedDefaults }
    }
    const value = parsed.data
    const goals = Object.fromEntries(Object.entries(value.goals ?? {})
      .filter(([key, goal]) => {
        const definition = goalDefinitions.find((item) => item.id === key)
        return definition && Number.isInteger(goal) && goal >= definition.min && goal <= definition.max
      }))
    return {
      ...persistedDefaults,
      ...value,
      schemaVersion: SCHEMA_VERSION,
      goals: { ...persistedDefaults.goals, ...goals },
    }
  } catch {
    if (raw) {
      try { localStorage.setItem(`${STORAGE_KEY}.corrupt`, raw) } catch { /* best effort */ }
    }
    return { ...persistedDefaults }
  }
}

export function savePersistedState(value: PersistedState): void {
  if (typeof localStorage === 'undefined') return
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
  } catch {
    // QuotaExceeded / privacy-mode storage: UI keeps working without persistence.
  }
}
