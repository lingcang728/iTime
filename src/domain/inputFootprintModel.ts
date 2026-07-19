import type { InputActivityPoint, InputKeyCount } from '../providers/inputActivity'

export function activeInputMinutes(history: InputActivityPoint[]): number {
  return history.filter((point) => point.keyStrokes > 0).length
}

export function estimatedWordsPerMinute(keyStrokes: number, activeMinutes: number): number {
  return activeMinutes > 0 ? keyStrokes / 5 / activeMinutes : 0
}

export function backspaceCorrectionRate(keyStrokes: number, keys: InputKeyCount[], detailAvailable: boolean): number | null {
  if (!detailAvailable) return null
  const backspaces = keys.find((item) => item.key === 'Backspace')?.count ?? 0
  return backspaces / Math.max(1, keyStrokes + backspaces)
}
