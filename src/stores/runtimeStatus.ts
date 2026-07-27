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
  state: 'preview' | 'loading' | 'empty' | 'ready' | 'degraded' | 'error'
}

export function runtimeSyncStatus(input: RuntimeSyncInput): RuntimeSyncStatus {
  if (!input.desktop) return { title: '预览数据', detail: '非本机实时记录', state: 'preview' }
  const failed = input.statuses.findIndex((status) => status === 'error')
  if (failed >= 0) {
    return {
      title: '部分数据读取失败',
      detail: input.messages[failed] || '请打开设置查看数据来源',
      state: 'error',
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
  if (input.statuses.some((status) => status === 'loading')) {
    return {
      title: '正在读取本机数据',
      detail: input.lastUpdatedAt === null ? '等待首次加载' : '正在刷新已显示记录',
      state: 'loading',
    }
  }
  if (input.lastUpdatedAt === null) {
    return { title: '等待本机数据', detail: '尚未完成一次刷新', state: 'loading' }
  }
  const activeStatuses = input.statuses.filter((status) => status !== 'disabled')
  if (activeStatuses.length > 0 && activeStatuses.every((status) => status === 'empty')) {
    return { title: '暂无本机记录', detail: '采集已连接，等待第一条记录', state: 'empty' }
  }
  const elapsed = Math.max(0, input.now - input.lastUpdatedAt)
  const detail = elapsed < 60_000
    ? '不到 1 分钟前'
    : `${Math.max(1, Math.floor(elapsed / 60_000))} 分钟前`
  return { title: '本机数据已刷新', detail, state: 'ready' }
}
