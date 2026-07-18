import { describe, expect, it } from 'vitest'
import { hasActivityData, hasInputData } from './dataAvailability'

describe('data availability states', () => {
  it('treats only activity states containing usable evidence as available', () => {
    expect(hasActivityData('preview')).toBe(true)
    expect(hasActivityData('ready')).toBe(true)
    expect(hasActivityData('degraded')).toBe(true)
    expect(hasActivityData('loading')).toBe(false)
    expect(hasActivityData('unavailable')).toBe(false)
  })

  it('does not turn empty input snapshots into available data', () => {
    expect(hasInputData('preview')).toBe(true)
    expect(hasInputData('ready')).toBe(true)
    expect(hasInputData('loading')).toBe(false)
    expect(hasInputData('unavailable')).toBe(false)
  })
})
