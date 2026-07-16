import { z } from 'zod'
import type {
  InputActivityPoint,
  InputActivityProvider,
  InputActivitySnapshot,
  InputGranularity,
} from './inputActivity'
import type { TimeRange } from '../domain/events'

const daySchema = z.object({
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  keyStrokes: z.number().int().nonnegative(),
  clicks: z.number().int().nonnegative(),
  mouseDistance: z.number().nonnegative(),
  scrollDistance: z.number().int().nonnegative(),
})

const keyStatsSchema = z.object({
  source: z.string().min(1),
  updatedAt: z.number().nonnegative(),
  today: z.object({
    date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
    keyStrokes: z.number().int().nonnegative(),
    leftClicks: z.number().int().nonnegative(),
    rightClicks: z.number().int().nonnegative(),
    mouseDistance: z.number().nonnegative(),
    scrollDistance: z.number().int().nonnegative(),
  }),
  history: z.array(daySchema),
  singleKeys: z.array(z.object({ key: z.string().min(1), count: z.number().int().positive() })),
  shortcuts: z.array(z.object({ key: z.string().min(1), count: z.number().int().positive() })),
  capabilities: z.object({
    historyGranularity: z.literal('day'),
    minuteDensity: z.literal(false),
    splitHistoricalClicks: z.literal(false),
    sensitiveSurfaceExclusion: z.literal(false),
    deleteByDate: z.literal(false),
    timezoneSemantics: z.literal('utc-date-bucket'),
  }),
})

export type KeyStatsWireSnapshot = z.infer<typeof keyStatsSchema>

function formatLocalDate(value: number): string {
  const date = new Date(value)
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, '0'),
    String(date.getDate()).padStart(2, '0'),
  ].join('-')
}

function emptyPoint(range: TimeRange): InputActivityPoint {
  return {
    ...range,
    keyStrokes: 0,
    leftClicks: null,
    rightClicks: null,
    combinedClicks: 0,
    mouseDistance: 0,
    scrollDistance: 0,
  }
}

export class KeyStatsInputActivityProvider implements InputActivityProvider {
  constructor(private readonly data: KeyStatsWireSnapshot) {}

  getAvailableDates(): string[] {
    return [...new Set([...this.data.history.map((day) => day.date), this.data.today.date])].sort()
  }

  getSnapshot(range: TimeRange, _granularity: InputGranularity = 'day'): InputActivitySnapshot {
    const date = formatLocalDate(range.start)
    const isToday = date === this.data.today.date
    const historical = this.data.history.find((day) => day.date === date)
    let point = emptyPoint(range)
    if (isToday) {
      const today = this.data.today
      point = {
        ...range,
        keyStrokes: today.keyStrokes,
        leftClicks: today.leftClicks,
        rightClicks: today.rightClicks,
        combinedClicks: today.leftClicks + today.rightClicks,
        mouseDistance: today.mouseDistance,
        scrollDistance: today.scrollDistance,
      }
    } else if (historical) {
      point = {
        ...range,
        keyStrokes: historical.keyStrokes,
        leftClicks: null,
        rightClicks: null,
        combinedClicks: historical.clicks,
        mouseDistance: historical.mouseDistance,
        scrollDistance: historical.scrollDistance,
      }
    }

    return {
      version: 'keystats-legacy-v1',
      range,
      source: this.data.source,
      sourceUpdatedAt: this.data.updatedAt,
      cumulative: point,
      history: isToday || historical ? [point] : [],
      singleKeys: isToday ? this.data.singleKeys : [],
      shortcuts: isToday
        ? this.data.shortcuts.map(({ key, count }) => ({ shortcut: key, count }))
        : [],
      capabilities: {
        ...this.data.capabilities,
        keyHeatmap: isToday,
        functionalShortcuts: isToday,
      },
    }
  }
}

export function parseKeyStatsSnapshot(value: unknown): KeyStatsWireSnapshot {
  return keyStatsSchema.parse(value)
}

export async function loadKeyStatsProvider(): Promise<KeyStatsInputActivityProvider> {
  const { invoke } = await import('@tauri-apps/api/core')
  const raw = await invoke<unknown>('get_key_stats_snapshot')
  return new KeyStatsInputActivityProvider(parseKeyStatsSnapshot(raw))
}
