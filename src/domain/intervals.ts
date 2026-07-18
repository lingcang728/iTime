import type { TimeRange } from './events'

export function isValidRange(range: TimeRange): boolean {
  return Number.isFinite(range.start) && Number.isFinite(range.end) && range.end > range.start
}

export function clipRange(range: TimeRange, boundary: TimeRange): TimeRange | null {
  const start = Math.max(range.start, boundary.start)
  const end = Math.min(range.end, boundary.end)
  return end > start ? { start, end } : null
}

export function mergeRanges(ranges: TimeRange[]): TimeRange[] {
  const sorted = ranges.filter(isValidRange).map((range) => ({ ...range })).sort((a, b) => a.start - b.start || a.end - b.end)
  if (!sorted.length) return []
  const merged: TimeRange[] = [sorted[0]]
  for (const current of sorted.slice(1)) {
    const previous = merged[merged.length - 1]
    if (current.start <= previous.end) previous.end = Math.max(previous.end, current.end)
    else merged.push(current)
  }
  return merged
}

export function coalesceRangesBy<T extends TimeRange>(
  ranges: T[],
  keyOf: (range: T) => string,
  maximumGap = 0,
): T[] {
  const sorted = ranges
    .filter(isValidRange)
    .map((range) => ({ ...range }))
    .sort((a, b) => a.start - b.start || a.end - b.end)
  const merged: T[] = []
  for (const current of sorted) {
    const previous = merged.at(-1)
    if (previous && keyOf(previous) === keyOf(current) && current.start <= previous.end + maximumGap) {
      previous.end = Math.max(previous.end, current.end)
    } else {
      merged.push(current)
    }
  }
  return merged
}

export function intersectRanges(left: TimeRange[], right: TimeRange[]): TimeRange[] {
  const a = mergeRanges(left)
  const b = mergeRanges(right)
  const result: TimeRange[] = []
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    const intersection = clipRange(a[i], b[j])
    if (intersection) result.push(intersection)
    if (a[i].end <= b[j].end) i += 1
    else j += 1
  }
  return result
}

export function durationOf(ranges: TimeRange[]): number {
  return mergeRanges(ranges).reduce((total, range) => total + range.end - range.start, 0)
}

export function summedDuration(ranges: TimeRange[]): number {
  return ranges.filter(isValidRange).reduce((total, range) => total + range.end - range.start, 0)
}

export function peakConcurrency(ranges: TimeRange[]): number {
  const points = ranges.filter(isValidRange).flatMap((range) => [
    { time: range.start, delta: 1 },
    { time: range.end, delta: -1 },
  ])
  points.sort((a, b) => a.time - b.time || a.delta - b.delta)
  let current = 0
  let peak = 0
  for (const point of points) {
    current += point.delta
    peak = Math.max(peak, current)
  }
  return peak
}

export interface ActivityWindow {
  range: TimeRange
  activity: number
}

export function bestActivityWindow(
  ranges: TimeRange[],
  boundary: TimeRange,
  windowDuration = 60 * 60_000,
  step = 30 * 60_000,
): ActivityWindow | null {
  if (!isValidRange(boundary) || windowDuration <= 0 || step <= 0) return null
  const candidates = ranges.flatMap((range) => {
    const clipped = clipRange(range, boundary)
    return clipped ? [clipped] : []
  })
  if (!candidates.length) return null
  const width = Math.min(windowDuration, boundary.end - boundary.start)
  const lastStart = boundary.end - width
  const starts: number[] = []
  for (let start = boundary.start; start <= lastStart; start += step) starts.push(start)
  if (starts.at(-1) !== lastStart) starts.push(lastStart)

  let best: ActivityWindow | null = null
  for (const start of starts) {
    const range = { start, end: start + width }
    const activity = candidates.reduce((total, candidate) => {
      const overlap = clipRange(candidate, range)
      return total + (overlap ? overlap.end - overlap.start : 0)
    }, 0)
    if (!best || activity > best.activity) best = { range, activity }
  }
  return best?.activity ? best : null
}

export interface ConcurrencyWindow {
  range: TimeRange
  concurrency: number
}

export function peakConcurrencyWindow(ranges: TimeRange[]): ConcurrencyWindow | null {
  const deltas = new Map<number, number>()
  for (const range of ranges.filter(isValidRange)) {
    deltas.set(range.start, (deltas.get(range.start) ?? 0) + 1)
    deltas.set(range.end, (deltas.get(range.end) ?? 0) - 1)
  }
  const points = [...deltas].sort(([left], [right]) => left - right)
  let current = 0
  let previous: number | null = null
  let best: ConcurrencyWindow | null = null
  for (const [time, delta] of points) {
    if (previous !== null && time > previous && current > 0) {
      const duration = time - previous
      const bestDuration = best ? best.range.end - best.range.start : 0
      if (!best || current > best.concurrency || (current === best.concurrency && duration > bestDuration)) {
        best = { range: { start: previous, end: time }, concurrency: current }
      }
    }
    current += delta
    previous = time
  }
  return best
}

export function splitByLocalDay(range: TimeRange): TimeRange[] {
  if (!isValidRange(range)) return []
  const result: TimeRange[] = []
  let cursor = range.start
  while (cursor < range.end) {
    const current = new Date(cursor)
    const nextDay = new Date(current.getFullYear(), current.getMonth(), current.getDate() + 1).getTime()
    const end = Math.min(range.end, nextDay)
    result.push({ start: cursor, end })
    cursor = end
  }
  return result
}
