import { describe, expect, it } from 'vitest'
import { buildFocusHeatmap } from './focusHeatmap'

describe('focus heatmap calendar', () => {
  it('maps only supplied activity evidence into the seven-week calendar', () => {
    const days = buildFocusHeatmap('2026-05-20', [
      { date: '2026-05-19', duration: 90 * 60_000 },
      { date: '2026-05-20', duration: 6 * 3_600_000 },
    ])

    expect(days).toHaveLength(49)
    expect(days[0]).toMatchObject({ date: '2026-04-06', weekday: 0, weekIndex: 0, available: false })
    expect(days.find((day) => day.date === '2026-05-19')).toMatchObject({ intensity: 2, available: true })
    expect(days.find((day) => day.date === '2026-05-20')).toMatchObject({ intensity: 5, available: true })
  })

  it('distinguishes a measured zero from a date without collection evidence', () => {
    const days = buildFocusHeatmap('2026-05-20', [{ date: '2026-05-18', duration: 0 }])

    expect(days.find((day) => day.date === '2026-05-18')).toMatchObject({ duration: 0, intensity: 0, available: true })
    expect(days.find((day) => day.date === '2026-05-17')).toMatchObject({ duration: null, intensity: 0, available: false })
  })
})
