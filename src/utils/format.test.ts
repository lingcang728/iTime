import { describe, expect, it } from 'vitest'
import { formatDuration } from './format'

describe('duration formatting', () => {
  it('does not label a positive sub-minute interval as zero', () => {
    expect(formatDuration(12_000, true)).toBe('< 1 分钟')
    expect(formatDuration(12_000)).toBe('不足1分钟')
    expect(formatDuration(0, true)).toBe('0 分钟')
  })
})
