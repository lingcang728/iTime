import type { AppDuration, DaySnapshot, StatValue } from '../../domain/events'
import type { FocusSample } from '../../data/focusHeatmap'

const hour = 3_600_000
const weekdayNames = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

export interface WeeklyDayPoint {
  date: string
  label: string
  note: string
  computer: number | null
  foreground: number | null
  ai: number | null
  input: number | null
}

export type WeeklyApp = AppDuration

export interface WeeklyAchievement {
  id: string
  title: string
  detail: string
  unlocked: boolean
  available: boolean
  progress: number
}

export interface WeeklySummary {
  rangeLabel: string
  days: WeeklyDayPoint[]
  topApps: WeeklyApp[]
  focusSamples: FocusSample[]
  bestDay: WeeklyDayPoint | null
  totalAttention: number | null
  totalInput: number | null
  peakInputDay: WeeklyDayPoint | null
  improvementPercent: number | null
  comparisonBasis: 'previousWeek' | 'peerDays' | null
  achievements: WeeklyAchievement[]
}

function dateKey(timestamp: number): string {
  const date = new Date(timestamp)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function dayPoint(day: DaySnapshot): WeeklyDayPoint {
  const date = new Date(day.range.start)
  const value = (metric: StatValue): number | null => metric.sources.includes('unavailable') ? null : metric.value
  return {
    date: dateKey(day.range.start),
    label: weekdayNames[date.getDay()],
    note: `${date.getMonth() + 1}/${date.getDate()}`,
    computer: value(day.computerActivity),
    foreground: value(day.foregroundActivity),
    ai: value(day.aiInteraction),
    input: value(day.inputKeyStrokes),
  }
}

function sumAvailable(values: Array<number | null>): number | null {
  const available = values.filter((value): value is number => value !== null)
  return available.length ? available.reduce((total, value) => total + value, 0) : null
}

function progress(value: number | null, target: number): number {
  return value === null ? 0 : Math.min(1, value / target)
}

function rangeLabel(days: WeeklyDayPoint[]): string {
  if (!days.length) return '本周'
  const first = new Date(`${days[0].date}T12:00:00`)
  const last = new Date(`${days.at(-1)?.date}T12:00:00`)
  return `${first.getMonth() + 1}月${first.getDate()}日 – ${last.getMonth() + 1}月${last.getDate()}日`
}

function aggregateApps(days: DaySnapshot[]): WeeklyApp[] {
  const apps = new Map<string, WeeklyApp>()
  for (const day of days) {
    for (const app of day.apps) {
      const current = apps.get(app.appId)
      if (current) current.duration += app.duration
      else apps.set(app.appId, { ...app })
    }
  }
  return [...apps.values()].sort((a, b) => b.duration - a.duration).slice(0, 10)
}

export function buildWeeklySummary(days: DaySnapshot[], previousDays?: DaySnapshot[]): WeeklySummary {
  const points = days.map(dayPoint)
  const topApps = aggregateApps(days)
  const totalAttention = sumAvailable(points.map((day) => day.foreground))
  const totalAi = sumAvailable(points.map((day) => day.ai))
  const totalInput = sumAvailable(points.map((day) => day.input))
  const bestDay = points
    .filter((day): day is WeeklyDayPoint & { foreground: number } => day.foreground !== null)
    .sort((a, b) => b.foreground - a.foreground)[0] ?? null
  const peakInputDay = points
    .filter((day): day is WeeklyDayPoint & { input: number } => day.input !== null && day.input > 0)
    .sort((a, b) => b.input - a.input)[0] ?? null
  const previousTotal = previousDays
    ? sumAvailable(previousDays.map((day) => dayPoint(day).foreground))
    : null
  const peerValues = points.flatMap((day) => day !== bestDay && day.foreground !== null ? [day.foreground] : [])
  const peerAverage = peerValues.length
    ? peerValues.reduce((total, value) => total + value, 0) / peerValues.length
    : null
  const hasPreviousBaseline = totalAttention !== null && previousTotal !== null && previousTotal > 0
  const hasPeerBaseline = bestDay !== null && peerAverage !== null && peerAverage > 0
  const improvementPercent = hasPreviousBaseline
    ? Math.round((totalAttention - previousTotal) / previousTotal * 100)
    : hasPeerBaseline
      ? Math.round((bestDay.foreground - peerAverage) / peerAverage * 100)
      : null
  const activeDays = points.filter((day) => (day.foreground ?? 0) >= 30 * 60_000).length

  return {
    rangeLabel: rangeLabel(points),
    days: points,
    topApps,
    focusSamples: points.map((day) => ({ date: day.date, duration: day.foreground })),
    bestDay,
    totalAttention,
    totalInput,
    peakInputDay,
    improvementPercent,
    comparisonBasis: hasPreviousBaseline ? 'previousWeek' : hasPeerBaseline ? 'peerDays' : null,
    achievements: [
      { id: 'focus', title: '深度专注', detail: '主动注意力累计 20 小时', unlocked: (totalAttention ?? 0) >= 20 * hour, available: totalAttention !== null, progress: progress(totalAttention, 20 * hour) },
      { id: 'rhythm', title: '稳定节奏', detail: '至少 5 天专注超过 30 分钟', unlocked: activeDays >= 5, available: totalAttention !== null, progress: Math.min(1, activeDays / 5) },
      { id: 'ai', title: 'AI 工具使用', detail: 'AI 前台活跃累计 8 小时', unlocked: (totalAi ?? 0) >= 8 * hour, available: totalAi !== null, progress: progress(totalAi, 8 * hour) },
      { id: 'apps', title: '应用版图', detail: '本周使用 8 个不同应用', unlocked: topApps.length >= 8, available: points.some((day) => day.foreground !== null), progress: Math.min(1, topApps.length / 8) },
    ],
  }
}
