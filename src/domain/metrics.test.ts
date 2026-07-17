import { describe, expect, it } from 'vitest'
import type { AiInteractionInterval, AiWorkInterval, DeviceStateInterval, ForegroundAppInterval, TimeEvent } from './events'
import { aggregateCategories, deriveDaySnapshot } from './metrics'
import { MockDataProvider } from '../providers/prototypeDataProvider'

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
    expect(result.aiInteraction.value).toBe(20)
    expect(result.aiEffective.value).toBe(80)
    expect(result.aiCoverage.value).toBe(60)
    expect(result.parallelOverlap.value).toBe(50)
    expect(result.maxConcurrency.value).toBe(2)
    expect(result.totalDuration.value).toBe(70)
    expect(result.aiLeverage.value).toBe(4)
    expect(result.parallelGain.value).toBeCloseTo(4 / 3)
  })

  it('returns an unavailable ratio when its denominator is zero', () => {
    const result = deriveDaySnapshot([], { start: 0, end: 100 })
    expect(result.aiLeverage.value).toBeNull()
    expect(result.parallelGain.value).toBeNull()
    expect(result.aiLeverage.reviewState).toBe('needsReview')
    expect(result.foregroundActivity.value).toBeNull()
  })

  it('does not turn partial evidence into real zeroes', () => {
    const range = { start: 0, end: 100 }
    const events: TimeEvent[] = [
      event<DeviceStateInterval>({ id: 'd', type: 'device', state: 'active', start: 0, end: 100, ...evidence }),
      event<AiInteractionInterval>({ id: 'i', type: 'aiInteraction', toolId: 'codex', toolName: 'Codex', start: 10, end: 30, ...evidence }),
    ]
    const result = deriveDaySnapshot(events, range)
    expect(result.computerActivity.value).toBe(100)
    expect(result.aiInteraction.value).toBe(20)
    expect(result.foregroundActivity.value).toBeNull()
    expect(result.aiEffective.value).toBeNull()
    expect(result.parallelOverlap.value).toBeNull()
    expect(result.totalDuration.value).toBeNull()
    expect(result.aiLeverage.value).toBeNull()
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

  it('aggregates application durations into category totals and shares', () => {
    const categories = aggregateCategories([
      { appId: 'code', appName: 'VS Code', category: '开发', color: '#4f80e8', duration: 40 },
      { appId: 'terminal', appName: 'Terminal', category: '开发', color: '#777', duration: 10 },
      { appId: 'chatgpt', appName: 'ChatGPT', category: 'AI 工具', color: '#7866d7', duration: 50 },
    ])
    expect(categories.reduce((sum, category) => sum + category.duration, 0)).toBe(100)
    expect(categories.reduce((sum, category) => sum + category.share, 0)).toBeCloseTo(1)
    expect(categories.find((category) => category.category === '开发')).toMatchObject({ duration: 50, share: 0.5, representative: { appId: 'code' } })
  })

  it('derives parallel overlap independently for each AI tool', () => {
    const range = { start: 0, end: 100 }
    const events: TimeEvent[] = [
      event<DeviceStateInterval>({ id: 'd', type: 'device', state: 'active', start: 0, end: 100, ...evidence }),
      event<ForegroundAppInterval>({ id: 'f', type: 'foreground', appId: 'code', appName: 'VS Code', category: '开发', color: '#4f80e8', start: 20, end: 80, ...evidence }),
      event<AiWorkInterval>({ id: 'a', type: 'aiWork', agentId: 'a', toolId: 'codex', toolName: 'Codex', taskId: 't', start: 0, end: 50, ...evidence }),
    ]
    const result = deriveDaySnapshot(events, range)
    expect(result.aiTools[0].parallelOverlapDuration).toBe(30)
  })

  it('keeps silent waiting outside effective agent work', () => {
    const day = new MockDataProvider().getDay('2026-05-20')
    const effective = day.aiTools.reduce((sum, tool) => sum + tool.effectiveDuration, 0)
    const waiting = day.aiTools.reduce((sum, tool) => sum + tool.silentWaitDuration, 0)
    expect(waiting).toBeGreaterThan(0)
    expect(effective).toBe(day.aiEffective.value)
    expect(effective + waiting).toBeGreaterThan(Number(day.aiEffective.value))
  })
})
