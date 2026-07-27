import { describe, expect, it } from 'vitest'
import { parseLocalDataStatus } from './localData'

describe('local data status contract', () => {
  it('accepts the supported permanent retention and degraded preview', () => {
    expect(parseLocalDataStatus({
      directory: 'C:\\Users\\test\\AppData\\Local\\iTime\\Data',
      retentionDays: null,
      fileCount: 2,
      sizeBytes: 512,
      lastWriteAt: 1000,
      activityRecords: 3,
      keyboardRecords: 4,
      skippedRecords: 1,
      startAt: 100,
      endAt: 200,
      health: 'degraded',
      message: '已跳过 1 条记录',
    })).toMatchObject({ retentionDays: null, health: 'degraded', skippedRecords: 1 })
  })

  it('rejects retention values the backend does not support', () => {
    expect(() => parseLocalDataStatus({
      directory: 'data',
      retentionDays: 30,
      fileCount: 0,
      sizeBytes: 0,
      lastWriteAt: null,
      activityRecords: 0,
      keyboardRecords: 0,
      skippedRecords: 0,
      startAt: null,
      endAt: null,
      health: 'empty',
      message: 'empty',
    })).toThrow()
  })
})
