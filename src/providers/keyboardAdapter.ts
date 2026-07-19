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

const keyboardSnapshotSchema = z.object({
  source: z.string().min(1),
  updatedAt: z.number().nonnegative(),
  skippedRecords: z.number().int().nonnegative(),
  buckets: z.array(z.unknown()),
  capabilities: z.object({
    contentCaptured: z.literal(false),
    keyIdentityCaptured: z.literal(false),
    directKeyCount: z.literal(true),
    granularity: z.literal('minute'),
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
    return [...new Set(this.data.buckets.map((bucket) => formatLocalDate(bucket.start)))].sort()
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
    return {
      version: 'itime-keyboard-v1',
      range,
      source: this.data.source,
      sourceUpdatedAt: this.data.updatedAt,
      cumulative: point(range.start, range.end, keyStrokes),
      history,
      singleKeys: [],
      shortcuts: [],
      capabilities: {
        historyGranularity: 'minute',
        minuteDensity: true,
        keyHeatmap: false,
        functionalShortcuts: false,
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
    basis: 'Windows 低级键盘钩子的字符键按下计数；不保存键值或输入内容',
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
