import { z } from 'zod'
import type { TimeDataset, TimeEvent, TimeRange } from '../domain/events'

const intervalSchema = z.object({
  version: z.literal(1),
  start: z.number().int().nonnegative(),
  end: z.number().int().positive(),
  deviceState: z.enum(['active', 'idle', 'unknown']),
  appId: z.string().nullable(),
  appName: z.string().nullable(),
  aiTool: z.boolean(),
}).refine((value) => value.end > value.start)

const snapshotSchema = z.object({
  source: z.string().min(1),
  updatedAt: z.number().nonnegative(),
  recordedFrom: z.number().int().nonnegative().nullable(),
  skippedRecords: z.number().int().nonnegative(),
  intervals: z.array(intervalSchema),
  capabilities: z.object({
    contentCaptured: z.literal(false),
    windowTitlesCaptured: z.literal(false),
    executablePathsCaptured: z.literal(false),
    historicalBackfill: z.literal(false),
    sessionStatesCaptured: z.literal(false),
    samplingIntervalSeconds: z.number().int().positive(),
  }),
  health: z.object({
    collectorRunning: z.boolean(),
    lastWriteAt: z.number().int().nonnegative().nullable(),
    lastError: z.string().nullable(),
  }),
})

export type ActivityWireSnapshot = z.infer<typeof snapshotSchema>

const categoryStyles: Record<string, { category: string; color: string }> = {
  ai: { category: 'AI 工具', color: '#806be1' },
  'vs code': { category: '开发', color: '#4f7ee8' },
  chrome: { category: '浏览', color: '#4bb97a' },
  'microsoft edge': { category: '浏览', color: '#48aeb0' },
  youtube: { category: '视频', color: '#e86f4f' },
}

function appStyle(appName: string, aiTool: boolean): { category: string; color: string } {
  if (aiTool) return categoryStyles.ai
  return categoryStyles[appName.toLocaleLowerCase()] ?? { category: '其他', color: '#7f91a8' }
}

function eventsForInterval(
  interval: ActivityWireSnapshot['intervals'][number],
  source: string,
  index: number,
): TimeEvent[] {
  const device = {
    id: `device:${interval.start}:${index}`,
    type: 'device' as const,
    start: interval.start,
    end: interval.end,
    state: interval.deviceState,
    source,
    accuracyLabel: 'precise' as const,
    basis: 'Windows 前台程序与最后输入时间的本机采样',
    confidence: 0.95,
    reviewState: 'confirmed' as const,
  }
  if (!interval.appId || !interval.appName) return [device]
  const style = appStyle(interval.appName, interval.aiTool)
  const foreground = {
    id: `foreground:${interval.start}:${index}`,
    type: 'foreground' as const,
    start: interval.start,
    end: interval.end,
    appId: interval.appId,
    appName: interval.appName,
    aiToolId: interval.aiTool ? interval.appId : undefined,
    ...style,
    source,
    accuracyLabel: 'precise' as const,
    basis: 'Windows 当前前台可执行程序；未读取窗口标题',
    confidence: 0.96,
    reviewState: 'confirmed' as const,
  }
  if (!interval.aiTool || interval.deviceState !== 'active') return [device, foreground]
  const aiEvidence = {
    source,
    accuracyLabel: 'estimated' as const,
    basis: 'AI 工具处于前台且设备活跃；不代表后台任务执行时长',
    confidence: 0.62,
    reviewState: 'needsReview' as const,
  }
  return [
    device,
    foreground,
    {
      id: `ai-work:${interval.start}:${index}`,
      type: 'aiWork',
      start: interval.start,
      end: interval.end,
      agentId: interval.appId,
      toolId: interval.appId,
      toolName: interval.appName,
      taskId: `foreground-session:${interval.start}`,
      ...aiEvidence,
    },
    {
      id: `ai-interaction:${interval.start}:${index}`,
      type: 'aiInteraction',
      start: interval.start,
      end: interval.end,
      toolId: interval.appId,
      toolName: interval.appName,
      ...aiEvidence,
    },
  ]
}

export function parseActivitySnapshot(value: unknown): ActivityWireSnapshot {
  return snapshotSchema.parse(value)
}

export function activityDataset(value: unknown): TimeDataset {
  const snapshot = parseActivitySnapshot(value)
  return {
    version: 'itime-local-activity-v1',
    events: snapshot.intervals.flatMap((interval, index) => eventsForInterval(interval, snapshot.source, index)),
  }
}

export async function loadActivityData(range: TimeRange): Promise<{
  dataset: TimeDataset
  snapshot: ActivityWireSnapshot
}> {
  const { invoke } = await import('@tauri-apps/api/core')
  const raw = await invoke<unknown>('get_activity_snapshot', { start: range.start, end: range.end })
  const snapshot = parseActivitySnapshot(raw)
  return { dataset: activityDataset(snapshot), snapshot }
}
