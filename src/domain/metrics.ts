import type {
  AccuracyLabel,
  AiInteractionInterval,
  AiToolSummary,
  AiWorkInterval,
  AppDuration,
  CategoryDuration,
  CalculationType,
  DaySnapshot,
  EventEvidence,
  ForegroundAppInterval,
  StatUnit,
  StatValue,
  TimeEvent,
  TimeRange,
} from './events'
import { clipRange, durationOf, intersectRanges, mergeRanges, peakConcurrency, summedDuration } from './intervals'

function overlaps(event: TimeRange, range: TimeRange): boolean {
  return event.start < range.end && event.end > range.start
}

function evidenceFor(events: EventEvidence[], fallback: string): Pick<StatValue, 'sources' | 'accuracyLabel' | 'basis' | 'confidence' | 'reviewState'> {
  if (!events.length) {
    return {
      sources: ['mock-provider'],
      accuracyLabel: 'estimated',
      basis: fallback,
      confidence: 0,
      reviewState: 'needsReview',
    }
  }
  const sources = [...new Set(events.map((event) => event.source))]
  const accuracyLabel: AccuracyLabel = events.some((event) => event.accuracyLabel === 'estimated') ? 'estimated' : 'precise'
  const reviewState = events.some((event) => event.reviewState === 'needsReview') ? 'needsReview' : 'confirmed'
  return {
    sources,
    accuracyLabel,
    basis: [...new Set(events.map((event) => event.basis))].join('；'),
    confidence: Math.min(...events.map((event) => event.confidence)),
    reviewState,
  }
}

function metric(value: number | null, unit: StatUnit, range: TimeRange, calculationType: CalculationType, evidence: EventEvidence[], fallback: string): StatValue {
  return { value, unit, range, calculationType, ...evidenceFor(evidence, fallback) }
}

type EventOfType<K extends TimeEvent['type']> = Extract<TimeEvent, { type: K }>

function clippedEvents<K extends TimeEvent['type']>(events: TimeEvent[], type: K, range: TimeRange): EventOfType<K>[] {
  return events
    .filter((event): event is EventOfType<K> => event.type === type && overlaps(event, range))
    .map((event) => {
      const clipped = clipRange(event, range)
      return clipped ? { ...event, ...clipped } : null
    })
    .filter((event): event is EventOfType<K> => Boolean(event))
}

function deriveApps(foreground: ForegroundAppInterval[], activeRanges: TimeRange[]): AppDuration[] {
  const totals = new Map<string, AppDuration>()
  for (const event of foreground) {
    const duration = durationOf(intersectRanges([event], activeRanges))
    if (!duration) continue
    const existing = totals.get(event.appId)
    if (existing) existing.duration += duration
    else totals.set(event.appId, { appId: event.appId, appName: event.appName, category: event.category, color: event.color, duration })
  }
  return [...totals.values()].sort((a, b) => b.duration - a.duration)
}

export function aggregateCategories(apps: AppDuration[]): CategoryDuration[] {
  const totalDuration = apps.reduce((total, app) => total + app.duration, 0)
  const categories = new Map<string, Omit<CategoryDuration, 'share'>>()

  for (const app of apps) {
    const current = categories.get(app.category)
    if (!current) {
      categories.set(app.category, { category: app.category, duration: app.duration, representative: app })
      continue
    }
    current.duration += app.duration
    if (app.duration > current.representative.duration) current.representative = app
  }

  return [...categories.values()]
    .map((category) => ({ ...category, share: totalDuration ? category.duration / totalDuration : 0 }))
    .sort((a, b) => b.duration - a.duration)
}

function deriveTools(work: AiWorkInterval[], interactions: AiInteractionInterval[]): AiToolSummary[] {
  const total = summedDuration(work)
  const ids = [...new Set(work.map((event) => event.toolId))]
  return ids.map((toolId) => {
    const workIntervals = work.filter((event) => event.toolId === toolId)
    const interactionIntervals = interactions.filter((event) => event.toolId === toolId)
    const effectiveDuration = summedDuration(workIntervals)
    const evidence = evidenceFor([...workIntervals, ...interactionIntervals], '尚未检测到工具区间')
    return {
      toolId,
      toolName: workIntervals[0].toolName,
      foregroundDuration: durationOf(interactionIntervals),
      effectiveDuration,
      taskCount: new Set(workIntervals.map((event) => event.taskId)).size,
      contribution: total ? effectiveDuration / total : 0,
      accuracyLabel: evidence.accuracyLabel,
      confidence: evidence.confidence,
      reviewState: evidence.reviewState,
      workIntervals,
      interactionIntervals,
    }
  }).sort((a, b) => b.effectiveDuration - a.effectiveDuration)
}

export function deriveDaySnapshot(events: TimeEvent[], range: TimeRange): DaySnapshot {
  const device = clippedEvents(events, 'device', range)
  const foreground = clippedEvents(events, 'foreground', range)
  const aiWork = clippedEvents(events, 'aiWork', range)
  const aiInteractions = clippedEvents(events, 'aiInteraction', range)
  const voice = clippedEvents(events, 'voice', range)
  const media = clippedEvents(events, 'media', range)
  const input = clippedEvents(events, 'input', range)
  const activeDevice = device.filter((event) => event.state === 'active')
  const availableDevice = device.filter((event) => event.state !== 'sleep')
  const activeRanges = mergeRanges(activeDevice)
  const foregroundRanges = intersectRanges(foreground, activeRanges)
  const aiRanges = aiWork.map(({ start, end }) => ({ start, end }))
  const aiCoverageRanges = mergeRanges(aiRanges)
  const interactionRanges = mergeRanges(aiInteractions)
  const foregroundDuration = durationOf(foregroundRanges)
  const effectiveDuration = summedDuration(aiRanges)
  const coverageDuration = durationOf(aiCoverageRanges)
  const interactionDuration = durationOf(interactionRanges)
  const parallelDuration = durationOf(intersectRanges(foregroundRanges, aiCoverageRanges))
  const aiLeverage = interactionDuration ? effectiveDuration / interactionDuration : null
  const parallelGain = coverageDuration ? effectiveDuration / coverageDuration : null
  const snapshotEvents = events.filter((event) => overlaps(event, range))

  return {
    range,
    computerActivity: metric(durationOf(availableDevice), 'milliseconds', range, 'union', availableDevice, '设备状态数据不足'),
    foregroundActivity: metric(foregroundDuration, 'milliseconds', range, 'intersection', [...foreground, ...activeDevice], '前台活动数据不足'),
    aiEffective: metric(effectiveDuration, 'milliseconds', range, 'sum', aiWork, 'AI 工具执行区间不足'),
    aiCoverage: metric(coverageDuration, 'milliseconds', range, 'union', aiWork, 'AI 工具执行区间不足'),
    parallelOverlap: metric(parallelDuration, 'milliseconds', range, 'intersection', [...foreground, ...activeDevice, ...aiWork], '并行区间不足'),
    maxConcurrency: metric(peakConcurrency(aiRanges), 'count', range, 'peak', aiWork, 'AI 工具执行区间不足'),
    totalDuration: metric(foregroundDuration + effectiveDuration, 'milliseconds', range, 'sum', [...foreground, ...activeDevice, ...aiWork], '前台或 AI 数据不足'),
    aiLeverage: metric(aiLeverage, 'ratio', range, 'ratio', [...aiWork, ...aiInteractions], '缺少 AI 前台交互区间'),
    parallelGain: metric(parallelGain, 'ratio', range, 'ratio', aiWork, '缺少 AI 覆盖区间'),
    voiceDuration: metric(summedDuration(voice), 'milliseconds', range, 'sum', voice, '语音输入数据不足'),
    mediaDuration: metric(summedDuration(media), 'milliseconds', range, 'sum', media, '媒体播放数据不足'),
    inputKeyStrokes: metric(input.reduce((total, bucket) => total + bucket.keyStrokes, 0), 'count', range, 'sum', input, '输入活动数据不足'),
    apps: deriveApps(foreground, activeRanges),
    aiTools: deriveTools(aiWork, aiInteractions),
    events: snapshotEvents,
  }
}
