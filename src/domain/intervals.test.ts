import { describe, expect, it } from 'vitest'
import {
  bestActivityWindow,
  coalesceRangesBy,
  durationOf,
  intersectRanges,
  mergeRanges,
  peakConcurrency,
  peakConcurrencyWindow,
  splitByLocalDay,
  summedDuration,
} from './intervals'

describe('interval algebra', () => {
  it('coalesces adjacent product slices without joining different identities', () => {
    const ranges = coalesceRangesBy([
      { start: 0, end: 10, appId: 'codex' },
      { start: 15, end: 20, appId: 'codex' },
      { start: 20, end: 30, appId: 'chrome' },
    ], (range) => range.appId, 5)
    expect(ranges).toEqual([
      { start: 0, end: 20, appId: 'codex' },
      { start: 20, end: 30, appId: 'chrome' },
    ])
  })

  it('merges touching and overlapping half-open ranges', () => {
    expect(mergeRanges([{ start: 0, end: 10 }, { start: 10, end: 20 }, { start: 5, end: 8 }])).toEqual([{ start: 0, end: 20 }])
  })

  it('calculates union, sum, and intersection independently', () => {
    const ranges = [{ start: 0, end: 10 }, { start: 5, end: 15 }]
    expect(durationOf(ranges)).toBe(15)
    expect(summedDuration(ranges)).toBe(20)
    expect(intersectRanges(ranges, [{ start: 8, end: 12 }])).toEqual([{ start: 8, end: 12 }])
  })

  it('processes ends before starts for peak concurrency', () => {
    expect(peakConcurrency([{ start: 0, end: 10 }, { start: 10, end: 20 }])).toBe(1)
    expect(peakConcurrency([{ start: 0, end: 15 }, { start: 5, end: 20 }, { start: 7, end: 9 }])).toBe(3)
  })

  it('finds the most active rolling hour using summed provider work', () => {
    const hour = 60 * 60_000
    const result = bestActivityWindow([
      { start: hour, end: 2 * hour },
      { start: 1.5 * hour, end: 2.5 * hour },
      { start: 5 * hour, end: 5.25 * hour },
    ], { start: 0, end: 6 * hour })
    expect(result).toEqual({
      range: { start: hour, end: 2 * hour },
      activity: 1.5 * hour,
    })
  })

  it('returns the longest period at peak concurrency', () => {
    expect(peakConcurrencyWindow([
      { start: 0, end: 10 },
      { start: 2, end: 8 },
      { start: 4, end: 7 },
      { start: 20, end: 30 },
      { start: 22, end: 29 },
      { start: 24, end: 28 },
    ])).toEqual({ range: { start: 24, end: 28 }, concurrency: 3 })
    expect(peakConcurrencyWindow([])).toBeNull()
  })

  it('splits a cross-midnight range at local day boundaries', () => {
    const start = new Date(2026, 4, 20, 23, 30).getTime()
    const end = new Date(2026, 4, 21, 1, 0).getTime()
    const parts = splitByLocalDay({ start, end })
    expect(parts).toHaveLength(2)
    expect(summedDuration(parts)).toBe(end - start)
  })
})
