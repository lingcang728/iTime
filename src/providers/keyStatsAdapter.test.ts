import { describe, expect, it } from 'vitest'
import { KeyStatsInputActivityProvider, parseKeyStatsSnapshot } from './keyStatsAdapter'

const raw = {
  source: 'iTime 本机输入记录',
  updatedAt: 123,
  today: {
    date: '2026-07-16', keyStrokes: 200, leftClicks: 20, rightClicks: 4,
    mouseDistance: 90.5, scrollDistance: 12,
  },
  history: [{
    date: '2026-07-15', keyStrokes: 100, clicks: 18,
    mouseDistance: 40.5, scrollDistance: 6,
  }],
  singleKeys: [{ key: 'Space', count: 40 }],
  shortcuts: [{ key: 'Ctrl + C', count: 3 }],
  capabilities: {
    historyGranularity: 'day', minuteDensity: false, splitHistoricalClicks: false,
    sensitiveSurfaceExclusion: false, deleteByDate: false, timezoneSemantics: 'utc-date-bucket',
  },
}

const rangeFor = (date: string) => ({
  start: new Date(`${date}T00:00:00`).getTime(),
  end: new Date(`${date}T24:00:00`).getTime(),
})

describe('KeyStatsInputActivityProvider', () => {
  it('preserves real today click split and key counts', () => {
    const provider = new KeyStatsInputActivityProvider(parseKeyStatsSnapshot(raw))
    const snapshot = provider.getSnapshot(rangeFor('2026-07-16'), 'hour')
    expect(snapshot.cumulative.leftClicks).toBe(20)
    expect(snapshot.cumulative.combinedClicks).toBe(24)
    expect(snapshot.singleKeys[0]).toEqual({ key: 'Space', count: 40 })
    expect(snapshot.capabilities.minuteDensity).toBe(false)
  })

  it('does not invent a left/right split or historical key heatmap', () => {
    const provider = new KeyStatsInputActivityProvider(parseKeyStatsSnapshot(raw))
    const snapshot = provider.getSnapshot(rangeFor('2026-07-15'))
    expect(snapshot.cumulative.leftClicks).toBeNull()
    expect(snapshot.cumulative.rightClicks).toBeNull()
    expect(snapshot.cumulative.combinedClicks).toBe(18)
    expect(snapshot.singleKeys).toEqual([])
    expect(snapshot.capabilities.keyHeatmap).toBe(false)
  })

  it('rejects a backend payload that overstates privacy capabilities', () => {
    const unsafe = structuredClone(raw)
    unsafe.capabilities.sensitiveSurfaceExclusion = true
    expect(() => parseKeyStatsSnapshot(unsafe)).toThrow()
  })
})
