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

