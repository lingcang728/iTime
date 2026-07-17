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

  it('keeps foreground AI activity separate from Provider execution evidence', () => {
    const dataset = activityDataset({
      ...base,
      intervals: [{
        version: 1, start: 1000, end: 2000, deviceState: 'active',
        appId: 'process:codex', appName: 'Codex', aiTool: true,
      }],
    })
    const interaction = dataset.events.find((event) => event.type === 'aiInteraction')
    expect(dataset.events.some((event) => event.type === 'aiWork')).toBe(false)
    expect(interaction?.accuracyLabel).toBe('estimated')
    expect(interaction?.reviewState).toBe('needsReview')
  })

  it('normalizes display names in records written by older collectors', () => {
    const dataset = activityDataset({
      ...base,
      intervals: [{
        version: 1, start: 1000, end: 2000, deviceState: 'active',
        appId: 'process:chrome', appName: 'chrome', aiTool: false,
      }],
    })
    const foreground = dataset.events.find((event) => event.type === 'foreground')
    expect(foreground).toMatchObject({ appName: 'Chrome', category: '浏览' })
  })

  it('rejects a payload claiming that content was captured', () => {
    expect(() => parseActivitySnapshot({
      ...base,
      capabilities: { ...base.capabilities, contentCaptured: true },
      intervals: [],
    })).toThrow()
  })

  it('keeps valid intervals when one stored record is malformed', () => {
    const snapshot = parseActivitySnapshot({
      ...base,
      intervals: [
        { version: 1, start: 1000, end: 2000, deviceState: 'active', appId: 'code', appName: 'Code', aiTool: false },
        { version: 1, start: 3000, end: 2000, deviceState: 'active', appId: 'bad', appName: 'Bad', aiTool: false },
      ],
    })
    expect(snapshot.intervals).toHaveLength(1)
    expect(snapshot.skippedRecords).toBe(1)
  })
})
