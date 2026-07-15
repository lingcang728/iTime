import { describe, expect, it } from 'vitest'
import type { AiInteractionInterval, AiWorkInterval, DeviceStateInterval, ForegroundAppInterval, TimeEvent } from './events'
import { deriveDaySnapshot } from './metrics'

const evidence = {
  source: 'test-fixture',
  accuracyLabel: 'precise' as const,
  basis: '结构化测试事件',
  confidence: 1,
  reviewState: 'confirmed' as const,
}

function event<T>(value: T): T { return value }

describe('daily metrics', () => {
  it('derives all AI metrics from one event collection', () => {
    const range = { start: 0, end: 100 }
    const events: TimeEvent[] = [
      event<DeviceStateInterval>({ id: 'd', type: 'device', state: 'active', start: 0, end: 100, ...evidence }),
      event<ForegroundAppInterval>({ id: 'f', type: 'foreground', appId: 'code', appName: 'VS Code', category: '开发', color: '#4f80e8', start: 0, end: 60, ...evidence }),
      event<AiWorkInterval>({ id: 'a1', type: 'aiWork', agentId: 'a1', toolId: 'codex', toolName: 'Codex', taskId: 't1', start: 10, end: 50, ...evidence }),
      event<AiWorkInterval>({ id: 'a2', type: 'aiWork', agentId: 'a2', toolId: 'claude', toolName: 'Claude Code', taskId: 't2', start: 30, end: 70, ...evidence }),
      event<AiInteractionInterval>({ id: 'i', type: 'aiInteraction', toolId: 'codex', toolName: 'Codex', start: 10, end: 30, ...evidence }),
    ]
    const result = deriveDaySnapshot(events, range)
    expect(result.foregroundActivity.value).toBe(60)
    expect(result.aiEffective.value).toBe(80)
    expect(result.aiCoverage.value).toBe(60)
    expect(result.parallelOverlap.value).toBe(50)
    expect(result.maxConcurrency.value).toBe(2)
    expect(result.totalDuration.value).toBe(140)
    expect(result.aiLeverage.value).toBe(4)
    expect(result.parallelGain.value).toBeCloseTo(4 / 3)
  })

  it('returns an unavailable ratio when its denominator is zero', () => {
    const result = deriveDaySnapshot([], { start: 0, end: 100 })
    expect(result.aiLeverage.value).toBeNull()
    expect(result.parallelGain.value).toBeNull()
    expect(result.aiLeverage.reviewState).toBe('needsReview')
  })

  it('propagates estimated and low-confidence evidence', () => {
    const estimated: AiWorkInterval = {
      id: 'a', type: 'aiWork', agentId: 'a', toolId: 'tool', toolName: 'Tool', taskId: 't', start: 0, end: 20,
      source: 'process-activity', accuracyLabel: 'estimated', basis: '进程活动推断', confidence: 0.62, reviewState: 'needsReview',
    }
    const result = deriveDaySnapshot([estimated], { start: 0, end: 100 })
    expect(result.aiEffective.accuracyLabel).toBe('estimated')
    expect(result.aiEffective.confidence).toBe(0.62)
    expect(result.aiEffective.reviewState).toBe('needsReview')
  })
})
