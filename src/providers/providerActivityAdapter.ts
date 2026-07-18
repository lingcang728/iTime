import { z } from 'zod'
import type { AiWorkInterval, TimeDataset, TimeRange } from '../domain/events'

const providerIntervalSchema = z.object({
  version: z.literal(1),
  start: z.number().int().nonnegative(),
  end: z.number().int().positive(),
  provider: z.enum(['codex', 'claude']),
  toolId: z.string().min(1),
  toolName: z.string().min(1),
  agentId: z.string().min(1),
  taskId: z.string().min(1),
  status: z.enum(['running', 'completed']),
  basis: z.string().min(1),
  confidence: z.number().min(0).max(1),
}).refine((value) => value.end > value.start)

const snapshotSchema = z.object({
  source: z.string().min(1),
  updatedAt: z.number().nonnegative(),
  scannedFiles: z.number().int().nonnegative(),
  skippedFiles: z.number().int().nonnegative(),
  intervals: z.array(z.unknown()),
  capabilities: z.object({
    contentCaptured: z.literal(false),
    codexTaskEvents: z.boolean(),
    claudeTurnEvents: z.boolean(),
  }),
})

type ProviderWireInterval = z.infer<typeof providerIntervalSchema>
export type ProviderActivitySnapshot = Omit<z.infer<typeof snapshotSchema>, 'intervals'> & {
  intervals: ProviderWireInterval[]
}

export function parseProviderActivitySnapshot(value: unknown): ProviderActivitySnapshot {
  const envelope = snapshotSchema.parse(value)
  const intervals = envelope.intervals.flatMap((candidate) => {
    const parsed = providerIntervalSchema.safeParse(candidate)
    return parsed.success ? [parsed.data] : []
  })
  return { ...envelope, intervals }
}

export function providerActivityDataset(value: unknown): TimeDataset {
  const snapshot = parseProviderActivitySnapshot(value)
  const events: AiWorkInterval[] = snapshot.intervals.map((interval, index) => ({
    id: `provider:${interval.provider}:${interval.taskId}:${index}`,
    type: 'aiWork',
    start: interval.start,
    end: interval.end,
    agentId: interval.agentId,
    toolId: interval.toolId,
    toolName: interval.toolName,
    taskId: interval.taskId,
    status: interval.status,
    source: `local-provider:${interval.provider}`,
    accuracyLabel: interval.confidence >= 0.95 ? 'precise' : 'estimated',
    basis: interval.basis,
    confidence: interval.confidence,
    reviewState: interval.confidence >= 0.95 ? 'confirmed' : 'needsReview',
  }))
  return { version: 'itime-local-provider-v1', events }
}

export async function loadProviderActivityData(range: TimeRange): Promise<{
  dataset: TimeDataset
  snapshot: ProviderActivitySnapshot
}> {
  const { invoke } = await import('@tauri-apps/api/core')
  const raw = await invoke<unknown>('get_provider_activity_snapshot', { start: range.start, end: range.end })
  const snapshot = parseProviderActivitySnapshot(raw)
  return { dataset: providerActivityDataset(snapshot), snapshot }
}
