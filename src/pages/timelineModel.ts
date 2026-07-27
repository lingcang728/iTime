import type { TimeRange } from '../domain/events'

export type TimelineRangeMode = 'day' | 'work'

export interface TimelineTick {
  label: string
  percent: number
}

export function localDateKey(timestamp: number): string {
  const date = new Date(timestamp)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

export function isSelectedLocalDay(selectedDate: string, timestamp: number): boolean {
  return selectedDate === localDateKey(timestamp)
}

export function timelineRange(day: TimeRange, mode: TimelineRangeMode): TimeRange {
  if (mode === 'day') return { ...day }
  const date = new Date(day.start)
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 9).getTime()
  const end = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 18).getTime()
  return {
    start: Math.max(day.start, start),
    end: Math.min(day.end, end),
  }
}

export function timelineTicks(mode: TimelineRangeMode): TimelineTick[] {
  const hours = mode === 'day'
    ? [0, 3, 6, 9, 12, 15, 18, 21, 24]
    : [9, 10, 11, 12, 13, 14, 15, 16, 17, 18]
  const first = hours[0]
  const span = Math.max(1, hours.at(-1)! - first)
  return hours.map((hour) => ({
    label: `${String(hour).padStart(2, '0')}:00`,
    percent: (hour - first) / span * 100,
  }))
}

export function timelineNowPercent(
  selectedDate: string,
  range: TimeRange,
  timestamp: number,
): number | null {
  if (!isSelectedLocalDay(selectedDate, timestamp) || timestamp < range.start || timestamp >= range.end) {
    return null
  }
  return (timestamp - range.start) / Math.max(1, range.end - range.start) * 100
}
