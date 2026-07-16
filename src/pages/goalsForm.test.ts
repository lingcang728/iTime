import { describe, expect, it } from 'vitest'
import { validateGoalDraft, validateQuietRange, type GoalDraft } from './goalsForm'

const validDraft: GoalDraft = { learning: '120', development: '180', ai: '90', continuous: '50' }

describe('goal form validation', () => {
  it('accepts integer targets inside each supported range', () => {
    expect(validateGoalDraft(validDraft)).toEqual({
      ok: true,
      values: { learning: 120, development: 180, ai: 90, continuous: 50 },
    })
  })

  it('rejects fractions and unsafe reminder thresholds', () => {
    expect(validateGoalDraft({ ...validDraft, learning: '12.5' })).toEqual({ ok: false, message: '学习时间需要填写整数分钟。' })
    expect(validateGoalDraft({ ...validDraft, continuous: '5' })).toEqual({ ok: false, message: '连续使用阈值需在 10—240 分钟之间。' })
  })

  it('supports cross-midnight quiet periods but rejects identical endpoints', () => {
    expect(validateQuietRange('22:30', '08:00').ok).toBe(true)
    expect(validateQuietRange('08:00', '08:00')).toEqual({ ok: false, message: '开始和结束时间不能相同。' })
  })
})
