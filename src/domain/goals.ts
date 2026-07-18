export type GoalId = 'learning' | 'development' | 'ai' | 'continuous'
export type GoalDraft = Record<GoalId, string>
export type GoalValues = Record<GoalId, number>

export const goalDefinitions: ReadonlyArray<{
  id: GoalId
  label: string
  hint: string
  min: number
  max: number
}> = [
  { id: 'learning', label: '学习时间', hint: '每天用于阅读、课程等学习活动', min: 15, max: 1_440 },
  { id: 'development', label: '开发时间', hint: '每天用于编程与开发工具', min: 15, max: 1_440 },
  { id: 'ai', label: 'AI 前台活跃', hint: '设备活跃且 AI 工具处于前台的时间', min: 15, max: 1_440 },
  { id: 'continuous', label: '连续使用阈值', hint: '达到这个时长后提醒短暂休息', min: 10, max: 240 },
]

export type ValidationResult<T> = { ok: true; values: T } | { ok: false; message: string }

export function validateGoalDraft(draft: GoalDraft): ValidationResult<GoalValues> {
  const values = {} as GoalValues
  for (const definition of goalDefinitions) {
    const value = Number(draft[definition.id])
    if (!Number.isInteger(value)) return { ok: false, message: `${definition.label}需要填写整数分钟。` }
    if (value < definition.min || value > definition.max) {
      return { ok: false, message: `${definition.label}需在 ${definition.min}—${definition.max} 分钟之间。` }
    }
    values[definition.id] = value
  }
  return { ok: true, values }
}

export function validateQuietRange(start: string, end: string): ValidationResult<null> {
  const validTime = /^([01]\d|2[0-3]):[0-5]\d$/
  if (!validTime.test(start) || !validTime.test(end)) return { ok: false, message: '请选择完整的开始和结束时间。' }
  if (start === end) return { ok: false, message: '开始和结束时间不能相同。' }
  return { ok: true, values: null }
}
