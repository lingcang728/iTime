import type {
  AiInteractionInterval,
  AiWorkInterval,
  DeviceStateInterval,
  ForegroundAppInterval,
  InputActivityMinuteBucket,
  MediaPlaybackInterval,
  TimeDataset,
  TimeEvent,
  VoiceInputInterval,
} from '../domain/events'

const minute = 60_000

const dayConfigs = [
  { date: '2026-05-14', screen: 366, foreground: 228, ai: 96, voice: 31, media: 38 },
  { date: '2026-05-15', screen: 432, foreground: 264, ai: 132, voice: 42, media: 52 },
  { date: '2026-05-16', screen: 516, foreground: 312, ai: 156, voice: 54, media: 63 },
  { date: '2026-05-17', screen: 318, foreground: 186, ai: 84, voice: 27, media: 47 },
  { date: '2026-05-18', screen: 468, foreground: 276, ai: 144, voice: 51, media: 58 },
  { date: '2026-05-19', screen: 390, foreground: 240, ai: 126, voice: 39, media: 50 },
  { date: '2026-05-20', screen: 408, foreground: 262, ai: 156, voice: 68, media: 43 },
]

const apps = [
  { id: 'vscode', name: 'VS Code', category: '开发', color: '#4f80e8', share: 0.32 },
  { id: 'chrome', name: 'Chrome', category: '学习', color: '#55b77a', share: 0.26 },
  { id: 'chatgpt', name: 'ChatGPT', category: 'AI 工具', color: '#7866d7', share: 0.19, aiToolId: 'chatgpt' },
  { id: 'typeless', name: 'Typeless', category: '语音输入', color: '#52b9b4', share: 0.12 },
  { id: 'youtube', name: 'YouTube', category: '视频', color: '#df7650', share: 0.07 },
  { id: 'explorer', name: '文件资源管理器', category: '系统工具', color: '#d69b40', share: 0.04 },
]

const aiTools = [
  { id: 'codex', name: 'Codex', share: 0.46, source: 'codex-structured-events', precise: true },
  { id: 'chatgpt', name: 'ChatGPT', share: 0.27, source: 'desktop-process-events', precise: false },
  { id: 'claude', name: 'Claude Code', share: 0.18, source: 'terminal-task-events', precise: true },
  { id: 'antigravity', name: 'AntiGravity', share: 0.09, source: 'window-activity-estimate', precise: false },
]

function at(date: string, hours: number, minutes = 0): number {
  return new Date(`${date}T${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:00`).getTime()
}

function nextDate(date: string): string {
  const value = new Date(`${date}T12:00:00`)
  value.setDate(value.getDate() + 1)
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

function precise(source: string, basis: string) {
  return { source, accuracyLabel: 'precise' as const, basis, confidence: 0.98, reviewState: 'confirmed' as const }
}

function estimated(source: string, basis: string, confidence = 0.74) {
  return { source, accuracyLabel: 'estimated' as const, basis, confidence, reviewState: 'needsReview' as const }
}

function distribute(total: number, shares: number[]): number[] {
  let used = 0
  return shares.map((share, index) => {
    const value = index === shares.length - 1 ? total - used : Math.floor(total * share)
    used += value
    return value
  })
}

function buildDay(config: typeof dayConfigs[number], dayIndex: number): TimeEvent[] {
  const result: TimeEvent[] = []
  const midnight = at(config.date, 0)
  const start = at(config.date, 8)
  const activeEnd = start + config.screen * minute
  const tomorrow = at(nextDate(config.date), 0)

  result.push(
    { id: `${config.date}-sleep-am`, type: 'device', state: 'sleep', start: midnight, end: start, ...precise('windows-power-events', 'Windows 睡眠与唤醒事件') } satisfies DeviceStateInterval,
    { id: `${config.date}-device-active`, type: 'device', state: 'active', start, end: activeEnd, ...precise('windows-session-events', 'Windows 会话与空闲状态') } satisfies DeviceStateInterval,
    { id: `${config.date}-sleep-pm`, type: 'device', state: 'sleep', start: activeEnd, end: tomorrow, ...precise('windows-power-events', 'Windows 睡眠与唤醒事件') } satisfies DeviceStateInterval,
  )

  const appMinutes = distribute(config.foreground, apps.map((app) => app.share))
  let appCursor = start + 20 * minute
  apps.forEach((app, index) => {
    const end = appCursor + appMinutes[index] * minute
    result.push({
      id: `${config.date}-app-${app.id}`,
      type: 'foreground',
      appId: app.id,
      appName: app.name,
      category: app.category,
      color: app.color,
      aiToolId: app.aiToolId,
      start: appCursor,
      end,
      ...precise('windows-foreground-events', '活动窗口与设备活跃区间交集'),
    } satisfies ForegroundAppInterval)
    appCursor = end + (5 + (index % 2) * 3) * minute
  })

  const workMinutes = distribute(config.ai, aiTools.map((tool) => tool.share))
  const aiStarts = [at(config.date, 9), at(config.date, 9, 28), at(config.date, 12, 50), at(config.date, 13, 2)]
  aiTools.forEach((tool, index) => {
    const eventEvidence = tool.precise
      ? precise(tool.source, '结构化任务开始与结束事件')
      : estimated(tool.source, '进程、窗口和文件活动综合推断', index === 1 ? 0.78 : 0.68)
    result.push({
      id: `${config.date}-ai-${tool.id}`,
      type: 'aiWork',
      agentId: `${tool.id}-${dayIndex}`,
      toolId: tool.id,
      toolName: tool.name,
      taskId: `${config.date}-${tool.id}-task`,
      start: aiStarts[index],
      end: aiStarts[index] + workMinutes[index] * minute,
      ...eventEvidence,
    } satisfies AiWorkInterval)
    const interactionMinutes = Math.max(5, Math.round(workMinutes[index] * (0.18 + index * 0.03)))
    result.push({
      id: `${config.date}-interaction-${tool.id}`,
      type: 'aiInteraction',
      toolId: tool.id,
      toolName: tool.name,
      start: aiStarts[index],
      end: aiStarts[index] + interactionMinutes * minute,
      ...precise('windows-foreground-events', 'AI 工具前台活动区间'),
    } satisfies AiInteractionInterval)
  })

  result.push({
    id: `${config.date}-voice`, type: 'voice', toolName: 'Typeless', start: at(config.date, 10, 35), end: at(config.date, 10, 35) + config.voice * minute,
    characters: Math.round(config.voice * 18.9), ...precise('voice-tool-session-events', '语音工具唤起至文字插入完成'),
  } satisfies VoiceInputInterval)
  result.push({
    id: `${config.date}-media`, type: 'media', appName: 'Chrome', awayPlayback: true, start: at(config.date, 14, 5), end: at(config.date, 14, 5) + config.media * minute,
    ...estimated('media-session-events', '媒体会话与窗口状态推断', 0.82),
  } satisfies MediaPlaybackInterval)

  for (let offset = 0; offset < config.screen; offset += 1) {
    const bucketStart = start + offset * minute
    const restMinute = (offset + dayIndex * 3) % 17 === 0
    const wave = ((offset * 7 + dayIndex * 11) % 23) + 7
    result.push({
      id: `${config.date}-input-${offset}`,
      type: 'input',
      start: bucketStart,
      end: bucketStart + minute,
      keyStrokes: restMinute ? 0 : wave,
      leftClicks: restMinute ? 0 : (offset * 3 + dayIndex) % 6,
      rightClicks: restMinute ? 0 : (offset + dayIndex) % 19 === 0 ? 1 : 0,
      mouseDistance: restMinute ? 0 : 1800 + ((offset * 97) % 9200),
      scrollDistance: restMinute ? 0 : (offset * 5 + dayIndex) % 47,
      ...precise('mock-input-minute-buckets', '按分钟聚合的输入活动，不含原始事件序列'),
    } satisfies InputActivityMinuteBucket)
  }

  return result
}

export const mockDataset: TimeDataset = {
  version: 'itime-events-v1',
  events: dayConfigs.flatMap(buildDay),
}

export const mockDates = dayConfigs.map((config) => config.date)

