import { describe, expect, it } from 'vitest'
import { activityDataset, parseActivitySnapshot } from './activityAdapter'

const base = {
  source: 'iTime 本机活动采样 · 从启用后记录',
  updatedAt: 123,
  recordedFrom: 1000,
  skippedRecords: 0,
  capabilities: {
    contentCaptured: false,
    windowTitlesCaptured: false,
    executablePathsCaptured: false,
    historicalBackfill: false,
    sessionStatesCaptured: false,
    samplingIntervalSeconds: 10,
  },
  health: { collectorRunning: true, lastWriteAt: 2000, lastError: null },
}

describe('activity adapter', () => {
  it('creates real device and foreground events without titles', () => {
    const dataset = activityDataset({
      ...base,
      intervals: [{
        version: 1, start: 1000, end: 2000, deviceState: 'active',
        appId: 'process:code', appName: 'Code', aiTool: false,
      }],
    })
    expect(dataset.events.map((event) => event.type)).toEqual(['device', 'foreground'])
    expect(JSON.stringify(dataset)).not.toContain('windowTitle')
  })

  it('marks foreground AI duration as an estimate that needs review', () => {
    const dataset = activityDataset({
      ...base,
      intervals: [{
        version: 1, start: 1000, end: 2000, deviceState: 'active',
        appId: 'process:codex', appName: 'Codex', aiTool: true,
      }],
    })
    const work = dataset.events.find((event) => event.type === 'aiWork')
    expect(work?.accuracyLabel).toBe('estimated')
    expect(work?.reviewState).toBe('needsReview')
  })

  it('rejects a payload claiming that content was captured', () => {
    expect(() => parseActivitySnapshot({
      ...base,
      capabilities: { ...base.capabilities, contentCaptured: true },
      intervals: [],
    })).toThrow()
  })
})
