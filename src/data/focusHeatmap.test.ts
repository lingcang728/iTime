import { describe, expect, it } from 'vitest'
import { buildFocusHeatmap } from './focusHeatmap'

describe('focus heatmap calendar', () => {
  it('maps 49 fixed days into seven Monday-to-Sunday week columns', () => {
    const days = buildFocusHeatmap('2026-05-20')
    expect(days).toHaveLength(49)
    days.forEach((day, index) => {
      expect(day.weekIndex).toBe(Math.floor(index / 7))
      expect(day.weekday).toBe(index % 7)
      expect(day.intensity).toBeGreaterThanOrEqual(0)
      expect(day.intensity).toBeLessThanOrEqual(5)
    })
    expect(days[0]).toMatchObject({ date: '2026-04-06', weekday: 0, weekIndex: 0 })
    expect(days.find((day) => day.date === '2026-05-20')).toMatchObject({ weekday: 2, weekIndex: 6, duration: 6.8 * 3_600_000 })
    expect(days.slice(-4).every((day) => day.duration === 0)).toBe(true)
  })
})
