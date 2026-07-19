import { describe, expect, it } from 'vitest'
import { dataProvider } from '../../providers/prototypeDataProvider'
import { buildWeeklySummary } from './weeklyModel'

describe('weekly summary', () => {
  it('derives dates, best day, apps and achievements from the supplied week', () => {
    const summary = buildWeeklySummary(dataProvider.getWeek('2026-05-20'))

    expect(summary.rangeLabel).toBe('5月14日 – 5月20日')
    expect(summary.bestDay?.label).toBe('周三')
    expect(summary.bestDay?.foreground).toBe(5.2 * 3_600_000)
    expect(summary.topApps).toHaveLength(6)
    expect(summary.topApps.map((app) => app.appId)).toEqual(expect.arrayContaining(['vscode', 'explorer']))
    expect(summary.focusSamples).toHaveLength(7)
    expect(summary.achievements).toHaveLength(4)
  })

  it('labels the best-day comparison as a peer-day baseline when prior-week data is absent', () => {
    const summary = buildWeeklySummary(dataProvider.getWeek('2026-05-20'))

    expect(summary.improvementPercent).toBeGreaterThan(0)
    expect(summary.comparisonBasis).toBe('peerDays')
  })
})
