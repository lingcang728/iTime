import { z } from 'zod'
import { isTauriRuntime } from './desktop'

const localDataStatusSchema = z.object({
  directory: z.string().min(1),
  retentionDays: z.union([z.literal(90), z.literal(365)]).nullable(),
  fileCount: z.number().int().nonnegative(),
  sizeBytes: z.number().int().nonnegative(),
  lastWriteAt: z.number().int().nonnegative().nullable(),
  activityRecords: z.number().int().nonnegative(),
  keyboardRecords: z.number().int().nonnegative(),
  skippedRecords: z.number().int().nonnegative(),
  startAt: z.number().int().nonnegative().nullable(),
  endAt: z.number().int().nonnegative().nullable(),
  health: z.enum(['ready', 'empty', 'degraded']),
  message: z.string().min(1),
})

const exportResultSchema = z.object({
  format: z.enum(['json', 'csv']),
  path: z.string().min(1),
  bytes: z.number().int().nonnegative(),
  activityRecords: z.number().int().nonnegative(),
  keyboardRecords: z.number().int().nonnegative(),
  skippedRecords: z.number().int().nonnegative(),
  startAt: z.number().int().nonnegative().nullable(),
  endAt: z.number().int().nonnegative().nullable(),
})

export type LocalDataStatus = z.infer<typeof localDataStatusSchema>
export type LocalDataExportResult = z.infer<typeof exportResultSchema>
export type DataRetentionDays = 90 | 365 | null
export type DataExportFormat = 'json' | 'csv'

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) throw new Error('本地数据控制仅在 iTime 桌面版中可用')
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(command, args)
}

export function parseLocalDataStatus(value: unknown): LocalDataStatus {
  return localDataStatusSchema.parse(value)
}

export async function loadLocalDataStatus(): Promise<LocalDataStatus> {
  return parseLocalDataStatus(await invokeDesktop<unknown>('get_local_data_status'))
}

export async function saveDataRetention(retentionDays: DataRetentionDays): Promise<LocalDataStatus> {
  return parseLocalDataStatus(await invokeDesktop<unknown>('set_data_retention', { retentionDays }))
}

export async function openLocalDataDirectory(): Promise<void> {
  await invokeDesktop('open_local_data_directory')
}

export async function exportLocalData(format: DataExportFormat): Promise<LocalDataExportResult> {
  return exportResultSchema.parse(await invokeDesktop<unknown>('export_local_data', { format }))
}

export async function clearAllLocalData(): Promise<LocalDataStatus> {
  return parseLocalDataStatus(await invokeDesktop<unknown>('clear_local_data', {
    confirmation: 'DELETE_ALL_LOCAL_DATA',
  }))
}
