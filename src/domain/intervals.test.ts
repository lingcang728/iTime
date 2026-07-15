import { describe, expect, it } from 'vitest'
import { durationOf, intersectRanges, mergeRanges, peakConcurrency, splitByLocalDay, summedDuration } from './intervals'

describe('interval algebra', () => {
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

  it('splits a cross-midnight range at local day boundaries', () => {
    const start = new Date(2026, 4, 20, 23, 30).getTime()
    const end = new Date(2026, 4, 21, 1, 0).getTime()
    const parts = splitByLocalDay({ start, end })
    expect(parts).toHaveLength(2)
    expect(summedDuration(parts)).toBe(end - start)
  })
})

