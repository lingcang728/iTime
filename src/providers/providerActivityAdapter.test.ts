import { describe, expect, it } from 'vitest'
import { parseProviderActivitySnapshot, providerActivityDataset } from './providerActivityAdapter'

const wire = {
  source: '已授权的 Codex 与 Claude Code 本机会话时间事件',
  status: 'ready',
  updatedAt: 1_752_800_000_000,
  scannedFiles: 2,
  skippedFiles: 0,
  consent: {
    version: 1,
    noticeSeen: true,
    codexEnabled: true,
    claudeEnabled: true,
  },
  diagnostics: {
    candidateFiles: 2,
    selectedFiles: 2,
    parsedFiles: 2,
    cacheHits: 0,
    badLines: 0,
    badEvents: 0,
    readFailures: 0,
    permissionFailures: 0,
  },
  intervals: [
    {
      version: 1,
      start: 1_752_800_000_000,
      end: 1_752_800_120_000,
      provider: 'codex',
      toolId: 'codex',
      toolName: 'Codex',
      agentId: 'agent-safe-id',
      taskId: 'task-safe-id',
      status: 'completed',
      basis: 'Codex 本机会话 task_started/task_complete 时间事件',
      confidence: 0.99,
    },
  ],
  capabilities: {
    contentCaptured: false,
    codexTaskEvents: true,
    claudeTurnEvents: true,
  },
}

describe('provider activity adapter', () => {
  it('maps bounded local provider events into AI work evidence', () => {
    const dataset = providerActivityDataset(wire)
    expect(dataset.events).toEqual([
      expect.objectContaining({
        type: 'aiWork',
        toolId: 'codex',
        taskId: 'task-safe-id',
        source: 'local-provider:codex',
        accuracyLabel: 'precise',
        confidence: 0.99,
      }),
    ])
  })

  it('rejects content-bearing capabilities and drops malformed intervals', () => {
    const parsed = parseProviderActivitySnapshot({
      ...wire,
      intervals: [...wire.intervals, { ...wire.intervals[0], start: wire.intervals[0].end }],
    })
    expect(parsed.intervals).toHaveLength(1)
    expect(() => parseProviderActivitySnapshot({
      ...wire,
      capabilities: { ...wire.capabilities, contentCaptured: true },
    })).toThrow()
  })

  it('requires the versioned consent and diagnostic envelope', () => {
    expect(() => parseProviderActivitySnapshot({
      ...wire,
      consent: { ...wire.consent, version: 2 },
    })).toThrow()
    expect(() => parseProviderActivitySnapshot({
      ...wire,
      diagnostics: { ...wire.diagnostics, permissionFailures: -1 },
    })).toThrow()
  })
})
