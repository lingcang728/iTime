import { describe, expect, it } from 'vitest'
import { runtimeSyncStatus } from './runtimeStatus'

describe('runtime sync status', () => {
  it('labels browser fixtures as preview instead of synchronized data', () => {
    expect(runtimeSyncStatus({
      desktop: false,
      now: 1_000,
      lastUpdatedAt: null,
      statuses: ['preview'],
      messages: ['浏览器预览数据'],
    })).toEqual({ title: '预览数据', detail: '非本机实时记录', state: 'preview' })
  })

  it('uses actual refresh time and surfaces source failures', () => {
    expect(runtimeSyncStatus({
      desktop: true,
      now: 121_000,
      lastUpdatedAt: 1_000,
      statuses: ['ready', 'ready'],
      messages: ['', ''],
    }).detail).toBe('2 分钟前')
    expect(runtimeSyncStatus({
      desktop: true,
      now: 1_000,
      lastUpdatedAt: null,
      statuses: ['ready', 'unavailable'],
      messages: ['', 'Provider 未授权'],
    })).toMatchObject({ title: '部分数据不可用', detail: 'Provider 未授权' })
  })
})
