export type AccuracyLabel = 'precise' | 'estimated'
export type ReviewState = 'confirmed' | 'needsReview'
export type AiToolStatus = 'running' | 'completed' | 'waiting'
export type CalculationType = 'sum' | 'union' | 'intersection' | 'peak' | 'ratio'
export type StatUnit = 'milliseconds' | 'count' | 'ratio' | 'pixels' | 'scrollUnits'

export interface EventEvidence {
  source: string
  accuracyLabel: AccuracyLabel
  basis: string
  confidence: number
  reviewState: ReviewState
}

export interface TimeRange {
  start: number
  end: number
}

interface BaseInterval extends TimeRange, EventEvidence {
  id: string
}

export interface DeviceStateInterval extends BaseInterval {
  type: 'device'
  state: 'active' | 'idle' | 'locked' | 'sleep' | 'unknown'
}

export interface ForegroundAppInterval extends BaseInterval {
  type: 'foreground'
  appId: string
  appName: string
  category: string
  color: string
  executablePath?: string
  aiToolId?: string
}

export interface AiWorkInterval extends BaseInterval {
  type: 'aiWork'
  agentId: string
  toolId: string
  toolName: string
  taskId: string
}

export interface AiInteractionInterval extends BaseInterval {
  type: 'aiInteraction'
  toolId: string
  toolName: string
}

export interface VoiceInputInterval extends BaseInterval {
  type: 'voice'
  toolName: string
  characters: number
}

export interface MediaPlaybackInterval extends BaseInterval {
  type: 'media'
  appName: string
  awayPlayback: boolean
}

export interface InputActivityMinuteBucket extends BaseInterval {
  type: 'input'
  keyStrokes: number
  leftClicks: number
  rightClicks: number
  mouseDistance: number
  scrollDistance: number
}

export type TimeEvent =
  | DeviceStateInterval
  | ForegroundAppInterval
  | AiWorkInterval
  | AiInteractionInterval
  | VoiceInputInterval
  | MediaPlaybackInterval
  | InputActivityMinuteBucket

export interface StatValue<T extends number = number> {
  value: T | null
  unit: StatUnit
  range: TimeRange
  calculationType: CalculationType
  sources: string[]
  accuracyLabel: AccuracyLabel
  basis: string
  confidence: number
  reviewState: ReviewState
}

export interface AppDuration {
  appId: string
  appName: string
  category: string
  color: string
  duration: number
}

export interface CategoryDuration {
  category: string
  duration: number
  share: number
  representative: AppDuration
}

export type TimelineKind = 'attention' | 'agent' | 'media' | 'other' | 'interaction' | 'waiting' | 'overlap'
export type TimelineVariant = 'solid' | 'outline' | 'hatched' | 'overlap'

export interface TimelineSegment extends TimeRange {
  kind?: TimelineKind
  variant?: TimelineVariant
  color?: string
  muted?: boolean
}

export interface AiToolSummary {
  toolId: string
  toolName: string
  status: AiToolStatus
  iconKey: string
  foregroundDuration: number
  effectiveDuration: number
  silentWaitDuration: number
  parallelOverlapDuration: number
  taskCount: number
  contribution: number
  accuracyLabel: AccuracyLabel
  confidence: number
  reviewState: ReviewState
  workIntervals: AiWorkInterval[]
  interactionIntervals: AiInteractionInterval[]
  waitIntervals: TimeRange[]
}

export interface DaySnapshot {
  range: TimeRange
  computerActivity: StatValue
  foregroundActivity: StatValue
  aiEffective: StatValue
  aiCoverage: StatValue
  parallelOverlap: StatValue
  maxConcurrency: StatValue
  totalDuration: StatValue
  aiLeverage: StatValue
  parallelGain: StatValue
  voiceDuration: StatValue
  mediaDuration: StatValue
  inputKeyStrokes: StatValue
  apps: AppDuration[]
  aiTools: AiToolSummary[]
  events: TimeEvent[]
}

export interface TimeDataset {
  version: string
  events: TimeEvent[]
}
