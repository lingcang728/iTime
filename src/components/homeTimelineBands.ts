import type { TimeRange, TimelineSegment } from '../domain/events'

export interface HomeTimelineBand {
  label: string
  range: TimeRange
  segments: TimelineSegment[]
}

const HOURS_PER_BAND = 4
const BAND_COUNT = 6
const HOUR_MS = 60 * 60 * 1000

export function buildHomeTimelineBands(range: TimeRange, segments: TimelineSegment[]): HomeTimelineBand[] {
  return Array.from({ length: BAND_COUNT }, (_, index) => {
    const start = range.start + index * HOURS_PER_BAND * HOUR_MS
    const end = start + HOURS_PER_BAND * HOUR_MS
    const fromHour = String(index * HOURS_PER_BAND).padStart(2, '0')
    const toHour = String((index + 1) * HOURS_PER_BAND).padStart(2, '0')
    return {
      label: `${fromHour}:00 – ${toHour}:00`,
      range: { start, end },
      segments: segments.flatMap((segment) => {
        const clippedStart = Math.max(start, segment.start)
        const clippedEnd = Math.min(end, segment.end)
        return clippedEnd > clippedStart ? [{ ...segment, start: clippedStart, end: clippedEnd }] : []
      }),
    }
  })
}
