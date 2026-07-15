import type { AiInteractionInterval, AiWorkInterval, DeviceStateInterval, EventEvidence, ForegroundAppInterval, InputActivityMinuteBucket, TimeDataset, TimeEvent } from '../domain/events'

const precise = { source: 'scenario-fixture', accuracyLabel: 'precise' as const, basis: '规范化场景事件', confidence: 1, reviewState: 'confirmed' as const }
const estimated = { source: 'scenario-estimate', accuracyLabel: 'estimated' as const, basis: '缺失区间前后的活动推断', confidence: 0.58, reviewState: 'needsReview' as const }

function dataset(events: TimeEvent[]): TimeDataset { return { version: 'scenario-v1', events } }
function device(id: string, start: number, end: number, state: DeviceStateInterval['state'] = 'active'): DeviceStateInterval { return { id, type: 'device', state, start, end, ...precise } }
function foreground(id: string, start: number, end: number, name = 'VS Code'): ForegroundAppInterval { return { id, type: 'foreground', appId: id, appName: name, category: '开发', color: '#4f80e8', start, end, ...precise } }
function ai(id: string, start: number, end: number, evidence: EventEvidence = precise): AiWorkInterval { return { id, type: 'aiWork', agentId: id, toolId: id, toolName: `Agent ${id}`, taskId: `task-${id}`, start, end, ...evidence } }
function interaction(id: string, start: number, end: number): AiInteractionInterval { return { id: `interaction-${id}`, type: 'aiInteraction', toolId: id, toolName: `Agent ${id}`, start, end, ...precise } }

const midnight = new Date(2026, 4, 20, 0, 0).getTime()
const nextMidnight = new Date(2026, 4, 21, 0, 0).getTime()
const minute = 60_000
const hour = 60 * minute

export const scenarioFixtures = {
  empty: dataset([]),
  foregroundOnly: dataset([device('device', midnight, nextMidnight), foreground('app', midnight + hour, midnight + 5 * hour)]),
  aiOnly: dataset([device('sleep', midnight, nextMidnight, 'sleep'), ai('codex', midnight + 2 * hour, midnight + 7 * hour), interaction('codex', midnight + 2 * hour, midnight + 2.5 * hour)]),
  threeAgents: dataset([device('device', midnight, nextMidnight), foreground('app', midnight + hour, midnight + 8 * hour), ai('a', midnight + 2 * hour, midnight + 6 * hour), ai('b', midnight + 3 * hour, midnight + 7 * hour), ai('c', midnight + 4 * hour, midnight + 5 * hour)]),
  overTwentyFourHours: dataset([device('device', midnight, nextMidnight), foreground('app', midnight, nextMidnight), ai('a', midnight, midnight + 12 * hour), ai('b', midnight, midnight + 12 * hour), ai('c', midnight, midnight + 12 * hour)]),
  crossMidnight: dataset([device('before', midnight + 23 * hour, nextMidnight, 'active'), device('after', nextMidnight, nextMidnight + 2 * hour, 'active'), foreground('cross-app', midnight + 23.5 * hour, nextMidnight + hour), ai('cross-ai', midnight + 23 * hour, nextMidnight + hour)]),
  sleepWake: dataset([device('active-am', midnight + 8 * hour, midnight + 10 * hour), device('sleep', midnight + 10 * hour, midnight + 12 * hour, 'sleep'), device('active-pm', midnight + 12 * hour, midnight + 15 * hour), foreground('before-sleep', midnight + 8 * hour, midnight + 11 * hour), foreground('after-wake', midnight + 12 * hour, midnight + 15 * hour)]),
  estimatedGap: dataset([device('device', midnight, nextMidnight), ai('estimated', midnight + 9 * hour, midnight + 11 * hour, estimated)]),
  lowConfidence: dataset([ai('review', midnight + hour, midnight + 2 * hour, { ...estimated, confidence: 0.31 })]),
  longNames: dataset([device('device', midnight, nextMidnight), foreground('long-app', midnight + hour, midnight + 2 * hour, '用于验证超长名称不会破坏布局的企业级集成开发环境与资料管理中心'), { ...ai('long-ai', midnight + 3 * hour, midnight + 4 * hour), toolName: '具有非常非常长名称的企业级人工智能代理编排与自动化工具' }]),
  input: dataset([device('device', midnight, nextMidnight), {
    id: 'input', type: 'input', start: midnight + hour, end: midnight + hour + minute,
    keyStrokes: 42, leftClicks: 7, rightClicks: 1, mouseDistance: 1200, scrollDistance: 16, ...precise,
  } satisfies InputActivityMinuteBucket]),
}

export const scenarioRange = { start: midnight, end: nextMidnight }
export const migrationFixtures = {
  notFound: { detected: false, version: null, partial: false },
  partial: { detected: true, version: 'legacy-v1', partial: true },
}
