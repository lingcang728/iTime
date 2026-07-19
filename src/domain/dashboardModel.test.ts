import { describe, expect, it } from 'vitest'
import { dataProvider } from '../providers/prototypeDataProvider'
import type { ForegroundAppInterval } from './events'
import { buildHomeComposition, countApplicationSwitches, relativeChange } from './dashboardModel'

const evidence = {
  source: 'test', accuracyLabel: 'precise' as const, basis: 'test', confidence: 1, reviewState: 'confirmed' as const,
}

describe('dashboard evidence model', () => {
  it('counts adjacent application changes instead of interval rows', () => {
    const events: ForegroundAppInterval[] = [
      { id: '1', type: 'foreground', start: 0, end: 10, appId: 'a', appName: 'A', category: '工作', color: '#000', ...evidence },
      { id: '2', type: 'foreground', start: 11, end: 20, appId: 'a', appName: 'A', category: '工作', color: '#000', ...evidence },
      { id: '3', type: 'foreground', start: 21, end: 30, appId: 'b', appName: 'B', category: '工作', color: '#000', ...evidence },
      { id: '4', type: 'foreground', start: 31, end: 40, appId: 'a', appName: 'A', category: '工作', color: '#000', ...evidence },
    ]
    expect(countApplicationSwitches(events)).toBe(2)
  })

  it('produces a non-overlapping computer-time composition', () => {
    const day = dataProvider.getDay('2026-05-20')
    const composition = buildHomeComposition(day)
    expect(composition.total).not.toBeNull()
    expect(composition.aiForeground + composition.otherDeepWork + composition.nonForeground).toBe(composition.total)
  })

  it('does not invent a comparison without a usable previous value', () => {
    expect(relativeChange(10, 5)).toBe(1)
    expect(relativeChange(10, 0)).toBeNull()
    expect(relativeChange(null, 5)).toBeNull()
  })
})
