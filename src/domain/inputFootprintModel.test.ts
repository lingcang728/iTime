import { describe, expect, it } from 'vitest'
import type { InputActivityPoint } from '../providers/inputActivity'
import { activeInputMinutes, backspaceCorrectionRate, estimatedWordsPerMinute } from './inputFootprintModel'

const point = (keyStrokes: number): InputActivityPoint => ({
  start: 0, end: 60_000, keyStrokes, leftClicks: null, rightClicks: null,
  combinedClicks: 0, mouseDistance: 0, scrollDistance: 0,
})

describe('input footprint metrics', () => {
  it('estimates WPM from five characters per word and active minutes only', () => {
    const history = [point(50), point(0), point(25)]
    const minutes = activeInputMinutes(history)
    expect(minutes).toBe(2)
    expect(estimatedWordsPerMinute(75, minutes)).toBe(7.5)
    expect(estimatedWordsPerMinute(75, 0)).toBe(0)
  })

  it('derives correction rate from day-level backspace evidence and does not backfill', () => {
    const keys = [{ key: 'Backspace', count: 10 }, { key: 'A', count: 90 }]
    expect(backspaceCorrectionRate(90, keys, true)).toBeCloseTo(0.1)
    expect(backspaceCorrectionRate(90, keys, false)).toBeNull()
  })
})
