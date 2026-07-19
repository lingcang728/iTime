import type { DaySnapshot, ForegroundAppInterval, TimeRange } from './events'
import { durationOf, intersectRanges, mergeRanges } from './intervals'

export interface HomeComposition {
  total: number | null
  aiForeground: number
  otherDeepWork: number
  nonForeground: number
}

export function countApplicationSwitches(events: ForegroundAppInterval[]): number {
  const ordered = [...events].sort((left, right) => left.start - right.start || left.end - right.end)
  let previous: string | null = null
  let switches = 0
  for (const event of ordered) {
    if (previous !== null && previous !== event.appId) switches += 1
    previous = event.appId
  }
  return switches
}

export function buildHomeComposition(day: DaySnapshot): HomeComposition {
  const total = day.computerActivity.value
  if (total === null) return { total: null, aiForeground: 0, otherDeepWork: 0, nonForeground: 0 }
  const foreground = day.events
    .filter((event): event is ForegroundAppInterval => event.type === 'foreground')
    .map(({ start, end }) => ({ start, end }))
  const aiForeground = day.events
    .filter((event) => event.type === 'aiInteraction')
    .map(({ start, end }) => ({ start, end }))
  const deepDuration = Math.min(total, day.foregroundActivity.value ?? 0)
  const aiDuration = Math.min(deepDuration, durationOf(intersectRanges(foreground, aiForeground)))
  return {
    total,
    aiForeground: aiDuration,
    otherDeepWork: Math.max(0, deepDuration - aiDuration),
    nonForeground: Math.max(0, total - deepDuration),
  }
}

export function dayTimelineSegments(day: DaySnapshot): {
  overview: TimeRange[]
  deepWork: TimeRange[]
  aiForeground: TimeRange[]
  nonForeground: TimeRange[]
} {
  const active = day.events
    .filter((event) => event.type === 'device' && (event.state === 'active' || event.state === 'idle'))
    .map(({ start, end }) => ({ start, end }))
  const deepWork = day.events
    .filter((event) => event.type === 'foreground')
    .map(({ start, end }) => ({ start, end }))
  const aiForeground = day.events
    .filter((event) => event.type === 'aiInteraction')
    .map(({ start, end }) => ({ start, end }))
  const activeMerged = mergeRanges(active)
  const deepMerged = mergeRanges(deepWork)
  return {
    overview: activeMerged,
    deepWork: deepMerged,
    aiForeground: mergeRanges(aiForeground),
    nonForeground: subtractRanges(activeMerged, deepMerged),
  }
}

function subtractRanges(base: TimeRange[], removal: TimeRange[]): TimeRange[] {
  const cuts = mergeRanges(removal)
  return mergeRanges(base).flatMap((range) => {
    const pieces: TimeRange[] = []
    let cursor = range.start
    for (const cut of cuts) {
      if (cut.end <= cursor || cut.start >= range.end) continue
      if (cut.start > cursor) pieces.push({ start: cursor, end: Math.min(cut.start, range.end) })
      cursor = Math.max(cursor, cut.end)
      if (cursor >= range.end) break
    }
    if (cursor < range.end) pieces.push({ start: cursor, end: range.end })
    return pieces
  })
}

export function relativeChange(current: number | null, previous: number | null): number | null {
  if (current === null || previous === null || previous === 0) return null
  return (current - previous) / previous
}
