import { z } from 'zod'
import type { ClosePreference, ThemeMode } from './appStore'

const STORAGE_KEY = 'itime-prototype-state'
const SCHEMA_VERSION = 2

export interface PersistedState {
  schemaVersion: typeof SCHEMA_VERSION
  theme: ThemeMode
  reminders: boolean
  closePreference: ClosePreference
  heatmapEnabled: boolean
  shortcutsEnabled: boolean
  quietStart: string
  quietEnd: string
  goals: Record<string, number>
  migrationState: 'notFound' | 'partial' | 'ready' | 'imported'
  deletedInputDates: string[]
}

export const persistedDefaults: PersistedState = {
  schemaVersion: SCHEMA_VERSION,
  theme: 'light',
  reminders: true,
  closePreference: 'ask',
  heatmapEnabled: true,
  shortcutsEnabled: true,
  quietStart: '22:00',
  quietEnd: '08:00',
  goals: { learning: 120, development: 180, ai: 180, continuous: 50 },
  migrationState: 'partial',
  deletedInputDates: [],
}

const storedSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  reminders: z.boolean().optional(),
  closePreference: z.enum(['ask', 'hide', 'quit']).optional(),
  heatmapEnabled: z.boolean().optional(),
  shortcutsEnabled: z.boolean().optional(),
  quietStart: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/).optional(),
  quietEnd: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/).optional(),
  goals: z.record(z.string(), z.number().finite().nonnegative()).optional(),
  migrationState: z.enum(['notFound', 'partial', 'ready', 'imported']).optional(),
  deletedInputDates: z.array(z.string().regex(/^\d{4}-\d{2}-\d{2}$/)).optional(),
}).strip()

export function loadPersistedState(): PersistedState {
  if (typeof localStorage === 'undefined') return { ...persistedDefaults }
  try {
    const parsed = storedSchema.safeParse(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}'))
    if (!parsed.success) return { ...persistedDefaults }
    const value = parsed.data
    const goals = Object.fromEntries(Object.entries(value.goals ?? {})
      .filter(([key]) => key in persistedDefaults.goals))
    return {
      ...persistedDefaults,
      ...value,
      schemaVersion: SCHEMA_VERSION,
      goals: { ...persistedDefaults.goals, ...goals },
    }
  } catch {
    return { ...persistedDefaults }
  }
}

export function savePersistedState(value: PersistedState): void {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
}
