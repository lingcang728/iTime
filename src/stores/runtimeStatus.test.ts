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
      statuses: ['ready', 'error'],
      messages: ['', 'Provider 未授权'],
    })).toMatchObject({ title: '部分数据读取失败', detail: 'Provider 未授权', state: 'error' })
  })

  it('distinguishes successful empty sources from failures', () => {
    expect(runtimeSyncStatus({
      desktop: true,
      now: 1_000,
      lastUpdatedAt: 900,
      statuses: ['empty', 'empty', 'disabled'],
      messages: ['', '', ''],
    })).toEqual({ title: '暂无本机记录', detail: '采集已连接，等待第一条记录', state: 'empty' })
  })

  it('shows background refreshes as loading even after an earlier success', () => {
    expect(runtimeSyncStatus({
      desktop: true,
      now: 2_000,
      lastUpdatedAt: 1_000,
      statuses: ['ready', 'loading', 'disabled'],
      messages: ['', '', ''],
    })).toEqual({ title: '正在读取本机数据', detail: '正在刷新已显示记录', state: 'loading' })
  })
})
