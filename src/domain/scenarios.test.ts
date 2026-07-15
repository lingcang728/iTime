import { describe, expect, it } from 'vitest'
import { migrationFixtures, scenarioFixtures, scenarioRange } from '../data/scenarioFixtures'
import { MockInputActivityProvider } from '../providers/inputActivity'
import { MockDataProvider } from '../providers/prototypeDataProvider'
import { deriveDaySnapshot } from './metrics'

const derive = (key: keyof typeof scenarioFixtures) => deriveDaySnapshot(scenarioFixtures[key].events, scenarioRange)

describe('required product scenarios', () => {
  it('represents a new user with unavailable ratios and review metadata', () => {
    const value = derive('empty')
    expect(value.foregroundActivity.value).toBe(0)
    expect(value.aiLeverage.value).toBeNull()
    expect(value.aiLeverage.reviewState).toBe('needsReview')
  })

  it('separates foreground-only and background-AI-only days', () => {
    expect(derive('foregroundOnly').foregroundActivity.value).toBe(4 * 3_600_000)
    expect(derive('foregroundOnly').aiEffective.value).toBe(0)
    expect(derive('aiOnly').foregroundActivity.value).toBe(0)
    expect(derive('aiOnly').aiEffective.value).toBe(5 * 3_600_000)
  })

  it('counts three simultaneous agents and allows total duration above 24 hours', () => {
    expect(derive('threeAgents').maxConcurrency.value).toBe(3)
    expect(derive('overTwentyFourHours').totalDuration.value).toBe(60 * 3_600_000)
  })

  it('clips cross-midnight tasks into the requested local day', () => {
    const firstDay = derive('crossMidnight')
    const secondRange = { start: scenarioRange.end, end: scenarioRange.end + 24 * 3_600_000 }
    const secondDay = deriveDaySnapshot(scenarioFixtures.crossMidnight.events, secondRange)
    expect(firstDay.aiEffective.value).toBe(3_600_000)
    expect(secondDay.aiEffective.value).toBe(3_600_000)
    expect(firstDay.foregroundActivity.value).toBe(30 * 60_000)
    expect(secondDay.foregroundActivity.value).toBe(3_600_000)
  })

  it('excludes foreground windows during sleep and resumes after wake', () => {
    expect(derive('sleepWake').foregroundActivity.value).toBe(5 * 3_600_000)
    expect(derive('sleepWake').computerActivity.value).toBe(5 * 3_600_000)
  })

  it('propagates missing-range estimates and low-confidence review state', () => {
    const gap = derive('estimatedGap')
    const low = derive('lowConfidence')
    expect(gap.aiEffective.accuracyLabel).toBe('estimated')
    expect(gap.aiEffective.sources).toContain('scenario-estimate')
    expect(low.aiEffective.confidence).toBe(0.31)
    expect(low.aiEffective.reviewState).toBe('needsReview')
  })

  it('exposes not-found and partial legacy migration states without reading files', () => {
    expect(migrationFixtures.notFound).toEqual({ detected: false, version: null, partial: false })
    expect(migrationFixtures.partial.detected).toBe(true)
    expect(migrationFixtures.partial.partial).toBe(true)
  })

  it('keeps long names intact for overflow-safe page rendering', () => {
    const value = derive('longNames')
    expect(value.apps[0].appName.length).toBeGreaterThan(20)
    expect(value.aiTools[0].toolName.length).toBeGreaterThan(20)
  })

  it('adapts minute input data into a standard snapshot', () => {
    const snapshot = new MockInputActivityProvider(scenarioFixtures.input).getSnapshot(scenarioRange)
    expect(snapshot.cumulative.keyStrokes).toBe(42)
    expect(snapshot.cumulative.leftClicks).toBe(7)
    expect(snapshot.source).toBe('mock-input-minute-buckets')
    expect(snapshot.capabilities.sensitiveSurfaceExclusion).toBe(true)
  })
})

describe('seven-page data consistency', () => {
  it('derives day, week, tool detail, timeline and input projections from one repository', () => {
    const provider = new MockDataProvider(scenarioFixtures.threeAgents)
    const date = '2026-05-20'
    const day = provider.getDay(date)
    const weekDay = provider.getWeek(date)[6]
    const detail = provider.getToolDetail(date, 'a')
    expect(weekDay.totalDuration.value).toBe(day.totalDuration.value)
    expect(detail?.effectiveDuration).toBe(day.aiTools.find((tool) => tool.toolId === 'a')?.effectiveDuration)
    expect(day.events).toBeDefined()
    expect(day.apps.reduce((sum, app) => sum + app.duration, 0)).toBe(day.foregroundActivity.value)
    expect(day.aiTools.reduce((sum, tool) => sum + tool.effectiveDuration, 0)).toBe(day.aiEffective.value)
  })
})
