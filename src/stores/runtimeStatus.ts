export interface RuntimeSyncInput {
  desktop: boolean
  now: number
  lastUpdatedAt: number | null
  statuses: string[]
  messages: string[]
}

export interface RuntimeSyncStatus {
  title: string
  detail: string
  state: 'preview' | 'loading' | 'ready' | 'degraded' | 'unavailable'
}

export function runtimeSyncStatus(input: RuntimeSyncInput): RuntimeSyncStatus {
  if (!input.desktop) return { title: '预览数据', detail: '非本机实时记录', state: 'preview' }
  const unavailable = input.statuses.findIndex((status) => status === 'unavailable')
  if (unavailable >= 0) {
    return {
      title: '部分数据不可用',
      detail: input.messages[unavailable] || '请打开设置查看数据来源',
      state: 'unavailable',
    }
  }
  const degraded = input.statuses.findIndex((status) => status === 'degraded')
  if (degraded >= 0) {
    return {
      title: '数据降级可用',
      detail: input.messages[degraded] || '部分记录无法读取',
      state: 'degraded',
    }
  }
  if (input.statuses.some((status) => status === 'loading') && input.lastUpdatedAt === null) {
    return { title: '正在读取本机数据', detail: '等待首次加载', state: 'loading' }
  }
  if (input.lastUpdatedAt === null) {
    return { title: '等待本机数据', detail: '尚未完成一次刷新', state: 'loading' }
  }
  const elapsed = Math.max(0, input.now - input.lastUpdatedAt)
  const detail = elapsed < 60_000
    ? '不到 1 分钟前'
    : `${Math.max(1, Math.floor(elapsed / 60_000))} 分钟前`
  return { title: '本机数据已刷新', detail, state: 'ready' }
}
