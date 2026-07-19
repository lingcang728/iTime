import { z } from 'zod'
import type { InputActivityMinuteBucket, TimeDataset, TimeRange } from '../domain/events'
import type {
  InputActivityPoint,
  InputActivityProvider,
  InputActivitySnapshot,
  InputGranularity,
} from './inputActivity'

const bucketSchema = z.object({
  version: z.literal(1),
  start: z.number().int().nonnegative(),
  end: z.number().int().positive(),
  keyStrokes: z.number().int().positive(),
}).refine((value) => value.end > value.start)

const detailDaySchema = z.object({
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  keys: z.array(z.object({
    key: z.string().min(1).max(24),
    count: z.number().int().positive(),
  })),
  shortcuts: z.array(z.object({
    shortcut: z.string().min(1).max(80),
    count: z.number().int().positive(),
  })),
})

const keyboardSnapshotSchema = z.object({
  source: z.string().min(1),
  updatedAt: z.number().nonnegative(),
  skippedRecords: z.number().int().nonnegative(),
  buckets: z.array(z.unknown()),
  detailStartedAt: z.number().int().nonnegative().nullable().default(null),
  detailDays: z.array(detailDaySchema).default([]),
  capabilities: z.object({
    contentCaptured: z.literal(false),
    sequenceCaptured: z.literal(false).default(false),
    keyIdentityCaptured: z.boolean(),
    shortcutCountsCaptured: z.boolean().default(false),
    directKeyCount: z.literal(true),
    granularity: z.literal('minute'),
    detailGranularity: z.literal('day').default('day'),
    timezoneSemantics: z.literal('local-time'),
    historicalBackfill: z.literal(false),
  }),
  health: z.object({
    collectorRunning: z.boolean(),
    lastWriteAt: z.number().int().nonnegative().nullable(),
    lastError: z.string().nullable(),
  }),
})

type KeyboardBucket = z.infer<typeof bucketSchema>
export type KeyboardWireSnapshot = Omit<z.infer<typeof keyboardSnapshotSchema>, 'buckets'> & {
  buckets: KeyboardBucket[]
}

function bucketStart(value: number, granularity: InputGranularity): number {
  const date = new Date(value)
  if (granularity === 'minute') return new Date(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours(), date.getMinutes()).getTime()
  if (granularity === 'hour') return new Date(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours()).getTime()
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()
}

function nextBucket(start: number, granularity: InputGranularity): number {
  const date = new Date(start)
  if (granularity === 'minute') date.setMinutes(date.getMinutes() + 1)
  else if (granularity === 'hour') date.setHours(date.getHours() + 1)
  else date.setDate(date.getDate() + 1)
  return date.getTime()
}

function point(start: number, end: number, keyStrokes: number): InputActivityPoint {
  return {
    start,
    end,
    keyStrokes,
    leftClicks: null,
    rightClicks: null,
    combinedClicks: 0,
    mouseDistance: 0,
    scrollDistance: 0,
  }
}

function formatLocalDate(value: number): string {
  const date = new Date(value)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

export class KeyboardInputActivityProvider implements InputActivityProvider {
  constructor(private readonly data: KeyboardWireSnapshot) {}

  getAvailableDates(): string[] {
    return [...new Set([
      ...this.data.buckets.map((bucket) => formatLocalDate(bucket.start)),
      ...this.data.detailDays.map((day) => day.date),
    ])].sort()
  }

  getSnapshot(range: TimeRange, granularity: InputGranularity = 'hour'): InputActivitySnapshot {
    const buckets = this.data.buckets.filter((bucket) => bucket.start < range.end && bucket.end > range.start)
    const groups = new Map<number, number>()
    for (const bucket of buckets) {
      const start = bucketStart(bucket.start, granularity)
      groups.set(start, (groups.get(start) ?? 0) + bucket.keyStrokes)
    }
    const history = [...groups.entries()]
      .sort(([left], [right]) => left - right)
      .map(([start, keyStrokes]) => point(start, nextBucket(start, granularity), keyStrokes))
    const keyStrokes = buckets.reduce((total, bucket) => total + bucket.keyStrokes, 0)
    const startDate = formatLocalDate(range.start)
    const endDate = formatLocalDate(Math.max(range.start, range.end - 1))
    const detailDays = this.data.detailDays.filter((day) => day.date >= startDate && day.date <= endDate)
    const keyCounts = new Map<string, number>()
    const shortcutCounts = new Map<string, number>()
    for (const day of detailDays) {
      for (const item of day.keys) keyCounts.set(item.key, (keyCounts.get(item.key) ?? 0) + item.count)
      for (const item of day.shortcuts) shortcutCounts.set(item.shortcut, (shortcutCounts.get(item.shortcut) ?? 0) + item.count)
    }
    const byCountThenName = <T extends { count: number }>(name: (value: T) => string) =>
      (left: T, right: T) => right.count - left.count || name(left).localeCompare(name(right))
    const singleKeys = [...keyCounts].map(([key, count]) => ({ key, count })).sort(byCountThenName((item) => item.key))
    const shortcuts = [...shortcutCounts].map(([shortcut, count]) => ({ shortcut, count })).sort(byCountThenName((item) => item.shortcut))
    const detailAvailable = this.data.detailStartedAt !== null && range.end > this.data.detailStartedAt
    return {
      version: 'itime-keyboard-v1',
      range,
      source: this.data.source,
      sourceUpdatedAt: this.data.updatedAt,
      detailAvailableFrom: this.data.detailStartedAt,
      cumulative: point(range.start, range.end, keyStrokes),
      history,
      singleKeys,
      shortcuts,
      capabilities: {
        historyGranularity: 'minute',
        detailGranularity: this.data.capabilities.keyIdentityCaptured ? 'day' : 'none',
        minuteDensity: true,
        keyHeatmap: detailAvailable && this.data.capabilities.keyIdentityCaptured,
        functionalShortcuts: detailAvailable && this.data.capabilities.shortcutCountsCaptured,
        contentCaptured: false,
        sequenceCaptured: false,
        keyIdentityCaptured: this.data.capabilities.keyIdentityCaptured,
        shortcutCountsCaptured: this.data.capabilities.shortcutCountsCaptured,
        sensitiveSurfaceExclusion: false,
        deleteByDate: false,
        splitHistoricalClicks: false,
        timezoneSemantics: 'local-day',
      },
    }
  }
}

export function parseKeyboardSnapshot(value: unknown): KeyboardWireSnapshot {
  const envelope = keyboardSnapshotSchema.parse(value)
  const buckets = envelope.buckets.flatMap((candidate) => {
    const parsed = bucketSchema.safeParse(candidate)
    return parsed.success ? [parsed.data] : []
  })
  return { ...envelope, buckets }
}

export function keyboardActivityDataset(value: unknown): TimeDataset {
  const snapshot = parseKeyboardSnapshot(value)
  const events: InputActivityMinuteBucket[] = snapshot.buckets.map((bucket, index) => ({
    id: `keyboard:${bucket.start}:${index}`,
    type: 'input',
    start: bucket.start,
    end: bucket.end,
    keyStrokes: bucket.keyStrokes,
    leftClicks: 0,
    rightClicks: 0,
    mouseDistance: 0,
    scrollDistance: 0,
    source: 'itime-keyboard-hook',
    accuracyLabel: 'estimated',
    basis: 'Windows 低级键盘钩子的分钟总量；键位与快捷键仅按本地日期聚合，不保存顺序、时间戳或输入内容',
    confidence: 0.94,
    reviewState: 'confirmed',
  }))
  return { version: 'itime-keyboard-v1', events }
}

export async function loadKeyboardData(range: TimeRange): Promise<{
  provider: KeyboardInputActivityProvider
  dataset: TimeDataset
  snapshot: KeyboardWireSnapshot
}> {
  const { invoke } = await import('@tauri-apps/api/core')
  const raw = await invoke<unknown>('get_keyboard_snapshot', { start: range.start, end: range.end })
  const snapshot = parseKeyboardSnapshot(raw)
  return {
    provider: new KeyboardInputActivityProvider(snapshot),
    dataset: keyboardActivityDataset(snapshot),
    snapshot,
  }
}
