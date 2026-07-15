import type { InputActivityMinuteBucket, TimeDataset, TimeRange } from '../domain/events'
import { mockDataset } from '../data/mockEvents'

export type InputGranularity = 'minute' | 'hour' | 'day'

export interface InputActivityCapabilities {
  minuteDensity: boolean
  keyHeatmap: boolean
  functionalShortcuts: boolean
  sensitiveSurfaceExclusion: boolean
  deleteByDate: boolean
}

export interface InputActivityPoint {
  start: number
  end: number
  keyStrokes: number
  leftClicks: number
  rightClicks: number
  mouseDistance: number
  scrollDistance: number
}

export interface InputKeyCount {
  key: string
  count: number
}

export interface InputShortcutCount {
  shortcut: string
  count: number
}

export interface InputActivitySnapshot {
  version: string
  range: TimeRange
  source: string
  cumulative: InputActivityPoint
  history: InputActivityPoint[]
  singleKeys: InputKeyCount[]
  shortcuts: InputShortcutCount[]
  capabilities: InputActivityCapabilities
}

export interface InputActivityProvider {
  getSnapshot(range: TimeRange, granularity?: InputGranularity): InputActivitySnapshot
}

export interface LegacyKeyStatsAdapter {
  inspect(): Promise<{ detected: boolean; version: string | null; partial: boolean }>
  createProvider(): Promise<InputActivityProvider>
}

function aggregate(points: InputActivityMinuteBucket[], start: number, end: number): InputActivityPoint {
  return points.reduce<InputActivityPoint>((total, point) => ({
    start,
    end,
    keyStrokes: total.keyStrokes + point.keyStrokes,
    leftClicks: total.leftClicks + point.leftClicks,
    rightClicks: total.rightClicks + point.rightClicks,
    mouseDistance: total.mouseDistance + point.mouseDistance,
    scrollDistance: total.scrollDistance + point.scrollDistance,
  }), { start, end, keyStrokes: 0, leftClicks: 0, rightClicks: 0, mouseDistance: 0, scrollDistance: 0 })
}

function bucketStart(value: number, granularity: InputGranularity): number {
  const date = new Date(value)
  if (granularity === 'minute') return new Date(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours(), date.getMinutes()).getTime()
  if (granularity === 'hour') return new Date(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours()).getTime()
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()
}

function bucketDuration(granularity: InputGranularity): number {
  if (granularity === 'minute') return 60_000
  if (granularity === 'hour') return 3_600_000
  return 86_400_000
}

export class MockInputActivityProvider implements InputActivityProvider {
  constructor(private dataset: TimeDataset = mockDataset) {}

  getSnapshot(range: TimeRange, granularity: InputGranularity = 'hour'): InputActivitySnapshot {
    const points = this.dataset.events.filter((event): event is InputActivityMinuteBucket => event.type === 'input' && event.start < range.end && event.end > range.start)
    const groups = new Map<number, InputActivityMinuteBucket[]>()
    for (const point of points) {
      const start = bucketStart(point.start, granularity)
      groups.set(start, [...(groups.get(start) ?? []), point])
    }
    const history = [...groups.entries()].sort(([a], [b]) => a - b).map(([start, values]) => aggregate(values, start, start + bucketDuration(granularity)))
    const cumulative = aggregate(points, range.start, range.end)
    const keys = ['Space', 'E', 'I', 'H', 'Shift', 'Backspace', 'Enter', 'Ctrl', 'A', 'S']
    const weights = [0.16, 0.14, 0.12, 0.1, 0.09, 0.08, 0.08, 0.07, 0.06, 0.05]
    return {
      version: 'itime-input-v1',
      range,
      source: 'mock-input-minute-buckets',
      cumulative,
      history,
      singleKeys: keys.map((key, index) => ({ key, count: Math.round(cumulative.keyStrokes * weights[index]) })),
      shortcuts: [
        { shortcut: 'Ctrl + C', count: Math.round(cumulative.keyStrokes * 0.018) },
        { shortcut: 'Ctrl + V', count: Math.round(cumulative.keyStrokes * 0.015) },
        { shortcut: 'Alt + Tab', count: Math.round(cumulative.keyStrokes * 0.009) },
        { shortcut: 'Ctrl + S', count: Math.round(cumulative.keyStrokes * 0.008) },
      ],
      capabilities: { minuteDensity: true, keyHeatmap: true, functionalShortcuts: true, sensitiveSurfaceExclusion: true, deleteByDate: true },
    }
  }
}

export const inputActivityProvider = new MockInputActivityProvider()

