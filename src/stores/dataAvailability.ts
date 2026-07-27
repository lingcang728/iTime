export type ActivityDataStatus = 'disabled' | 'loading' | 'preview' | 'empty' | 'ready' | 'degraded' | 'error'
export type InputDataStatus = 'loading' | 'preview' | 'empty' | 'ready' | 'degraded' | 'error'

export function hasActivityData(status: ActivityDataStatus): boolean {
  return status === 'preview' || status === 'ready' || status === 'degraded'
}

export function hasInputData(status: InputDataStatus): boolean {
  return status === 'preview' || status === 'ready' || status === 'degraded'
}
