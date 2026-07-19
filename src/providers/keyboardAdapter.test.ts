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
  capabilities: {
    contentCaptured: false,
    keyIdentityCaptured: false,
    directKeyCount: true,
    granularity: 'minute',
    timezoneSemantics: 'local-time',
    historicalBackfill: false,
  },
  health: { collectorRunning: true, lastWriteAt: 3 * minute, lastError: null },
}

describe('native keyboard adapter', () => {
  it('aggregates direct key counts without inventing mouse or key identity data', () => {
    const provider = new KeyboardInputActivityProvider(parseKeyboardSnapshot(raw))
    const snapshot = provider.getSnapshot({ start: 0, end: 4 * minute }, 'hour')
    expect(snapshot.cumulative.keyStrokes).toBe(11)
    expect(snapshot.cumulative.combinedClicks).toBe(0)
    expect(snapshot.singleKeys).toEqual([])
    expect(snapshot.shortcuts).toEqual([])
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

  it('rejects any wire capability that claims content or key identity capture', () => {
    expect(() => parseKeyboardSnapshot({
      ...raw,
      capabilities: { ...raw.capabilities, contentCaptured: true },
    })).toThrow()
    expect(() => parseKeyboardSnapshot({
      ...raw,
      capabilities: { ...raw.capabilities, keyIdentityCaptured: true },
    })).toThrow()
  })
})
