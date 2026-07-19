import { describe, expect, it } from 'vitest'
import { KeyboardInputActivityProvider, keyboardActivityDataset, parseKeyboardSnapshot } from './keyboardAdapter'

const minute = 60_000
const raw = {
  source: 'iTime Windows 键盘字符键计数',
  updatedAt: 3 * minute,
  skippedRecords: 0,
  buckets: [
    { version: 1, start: minute, end: 2 * minute, keyStrokes: 4 },
    { version: 1, start: 2 * minute, end: 3 * minute, keyStrokes: 7 },
  ],
  detailStartedAt: minute,
  detailDays: [{
    date: '1970-01-01',
    keys: [{ key: 'A', count: 6 }, { key: 'Backspace', count: 2 }],
    shortcuts: [{ shortcut: 'Ctrl + C', count: 3 }],
  }],
  capabilities: {
    contentCaptured: false,
    sequenceCaptured: false,
    keyIdentityCaptured: true,
    shortcutCountsCaptured: true,
    directKeyCount: true,
    granularity: 'minute',
    detailGranularity: 'day',
    timezoneSemantics: 'local-time',
    historicalBackfill: false,
  },
  health: { collectorRunning: true, lastWriteAt: 3 * minute, lastError: null },
}

describe('native keyboard adapter', () => {
  it('aggregates minute totals and day-level key evidence without inventing mouse data', () => {
    const provider = new KeyboardInputActivityProvider(parseKeyboardSnapshot(raw))
    const snapshot = provider.getSnapshot({ start: 0, end: 4 * minute }, 'hour')
    expect(snapshot.cumulative.keyStrokes).toBe(11)
    expect(snapshot.cumulative.combinedClicks).toBe(0)
    expect(snapshot.singleKeys).toEqual([{ key: 'A', count: 6 }, { key: 'Backspace', count: 2 }])
    expect(snapshot.shortcuts).toEqual([{ shortcut: 'Ctrl + C', count: 3 }])
    expect(snapshot.capabilities.sequenceCaptured).toBe(false)
  })

  it('maps minute counts into weekly metric events', () => {
    const dataset = keyboardActivityDataset(raw)
    expect(dataset.events).toHaveLength(2)
    expect(dataset.events[0]).toEqual(expect.objectContaining({
      type: 'input',
      keyStrokes: 4,
      source: 'itime-keyboard-hook',
    }))
  })

  it('rejects any wire capability that claims content or sequence capture', () => {
    expect(() => parseKeyboardSnapshot({
      ...raw,
      capabilities: { ...raw.capabilities, contentCaptured: true },
    })).toThrow()
    expect(() => parseKeyboardSnapshot({
      ...raw,
      capabilities: { ...raw.capabilities, sequenceCaptured: true },
    })).toThrow()
  })

  it('keeps legacy snapshots compatible and marks pre-upgrade detail unavailable', () => {
    const legacy = {
      ...raw,
      detailStartedAt: undefined,
      detailDays: undefined,
      capabilities: {
        contentCaptured: false,
        keyIdentityCaptured: false,
        directKeyCount: true,
        granularity: 'minute',
        timezoneSemantics: 'local-time',
        historicalBackfill: false,
      },
    }
    const provider = new KeyboardInputActivityProvider(parseKeyboardSnapshot(legacy))
    const snapshot = provider.getSnapshot({ start: 0, end: 4 * minute }, 'hour')
    expect(snapshot.cumulative.keyStrokes).toBe(11)
    expect(snapshot.detailAvailableFrom).toBeNull()
    expect(snapshot.capabilities.keyHeatmap).toBe(false)
    expect(snapshot.singleKeys).toEqual([])
  })
})
