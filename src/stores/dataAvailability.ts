export type ActivityDataStatus = 'loading' | 'preview' | 'ready' | 'degraded' | 'unavailable'
export type InputDataStatus = 'loading' | 'preview' | 'ready' | 'unavailable'

export function hasActivityData(status: ActivityDataStatus): boolean {
  return status === 'preview' || status === 'ready' || status === 'degraded'
}

export function hasInputData(status: InputDataStatus): boolean {
  return status === 'preview' || status === 'ready'
}
