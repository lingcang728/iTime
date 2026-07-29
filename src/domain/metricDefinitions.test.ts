import { describe, expect, it } from 'vitest'
import { comparisonLabel, metricDefinitions, metricInfo } from './metricDefinitions'

describe('metric definitions', () => {
  it('keeps a complete user-facing definition for each shared metric', () => {
    for (const definition of Object.values(metricDefinitions)) {
      expect(definition.name).not.toBe('')
      expect(definition.formula).not.toBe('')
      expect(definition.window).not.toBe('')
      expect(definition.missing).not.toBe('')
    }
    expect(metricInfo('foregroundSwitches')).toContain('相邻前台应用变化次数')
  })

  it('never converts a missing comparison baseline into zero', () => {
    expect(comparisonLabel(60, null, String)).toBe('暂无对比')
    expect(comparisonLabel(null, 20, String)).toBe('当前暂无数据')
    expect(comparisonLabel(30, 20, (value) => `${value} 分钟`)).toBe('较昨日 +10 分钟')
    expect(comparisonLabel(20, 30, (value) => `${value} 分钟`)).toBe('较昨日 −10 分钟')
  })
})
