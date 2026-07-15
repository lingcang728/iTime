import type { AiToolSummary, DaySnapshot, TimeDataset, TimeEvent, TimeRange } from '../domain/events'
import { deriveDaySnapshot } from '../domain/metrics'
import { mockDataset, mockToolRuntime } from '../data/mockEvents'

const minute = 60_000

export interface AiToolDetail extends AiToolSummary {
  detectionBasis: string[]
  pendingRecords: TimeEvent[]
}

export interface PrototypeDataProvider {
  getDay(date: string): DaySnapshot
  getRange(range: TimeRange): DaySnapshot
  getWeek(endDate: string): DaySnapshot[]
  getToolDetail(date: string, toolId: string): AiToolDetail | null
  subscribe(listener: () => void): () => void
}

function dayRange(date: string): TimeRange {
  const start = new Date(`${date}T00:00:00`).getTime()
  const endValue = new Date(`${date}T12:00:00`)
  endValue.setDate(endValue.getDate() + 1)
  const end = new Date(endValue.getFullYear(), endValue.getMonth(), endValue.getDate()).getTime()
  return { start, end }
}

function formatDate(value: Date): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

export class MockDataProvider implements PrototypeDataProvider {
  private listeners = new Set<() => void>()

  constructor(private dataset: TimeDataset = mockDataset) {}

  getDay(date: string): DaySnapshot {
    return this.getRange(dayRange(date))
  }

  getRange(range: TimeRange): DaySnapshot {
    const snapshot = deriveDaySnapshot(this.dataset.events, range)
    return {
      ...snapshot,
      aiTools: snapshot.aiTools.map((tool) => {
        const runtime = mockToolRuntime[tool.toolId]
        if (!runtime) return tool
        const requestedWait = runtime.silentWaitMinutes * minute
        const lastWorkEnd = Math.max(...tool.workIntervals.map((event) => event.end))
        const waitStart = Math.min(range.end, lastWorkEnd + 5 * minute)
        const waitEnd = Math.min(range.end, waitStart + requestedWait)
        const silentWaitDuration = Math.max(0, waitEnd - waitStart)
        return {
          ...tool,
          status: runtime.status,
          iconKey: runtime.iconKey,
          silentWaitDuration,
          waitIntervals: silentWaitDuration ? [{ start: waitStart, end: waitEnd }] : [],
        }
      }),
    }
  }

  getWeek(endDate: string): DaySnapshot[] {
    const end = new Date(`${endDate}T12:00:00`)
    return Array.from({ length: 7 }, (_, index) => {
      const date = new Date(end)
      date.setDate(end.getDate() - (6 - index))
      return this.getDay(formatDate(date))
    })
  }

  getToolDetail(date: string, toolId: string): AiToolDetail | null {
    const summary = this.getDay(date).aiTools.find((tool) => tool.toolId === toolId)
    if (!summary) return null
    const intervals = [...summary.workIntervals, ...summary.interactionIntervals]
    return {
      ...summary,
      detectionBasis: [
        ...new Set(intervals.map((event) => event.basis)),
        ...(summary.silentWaitDuration ? ['静默等待来自独立状态演示数据，不计入有效代理工时'] : []),
      ],
      pendingRecords: intervals.filter((event) => event.reviewState === 'needsReview'),
    }
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
}

export const dataProvider = new MockDataProvider()
export { dayRange }
