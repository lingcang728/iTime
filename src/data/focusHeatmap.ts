export interface FocusSample {
  date: string
  duration: number | null
}

export interface HeatmapDay {
  date: string
  weekday: number
  weekIndex: number
  duration: number | null
  intensity: number
  available: boolean
}

const hour = 3_600_000

function formatDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function intensityFor(duration: number | null): number {
  if (duration === null || duration <= 0) return 0
  const hours = duration / hour
  if (hours < 1) return 1
  if (hours < 2) return 2
  if (hours < 4) return 3
  if (hours < 6) return 4
  return 5
}

export function buildFocusHeatmap(endDate: string, samples: FocusSample[]): HeatmapDay[] {
  const selected = new Date(`${endDate}T12:00:00`)
  const mondayOffset = (selected.getDay() + 6) % 7
  const currentMonday = new Date(selected)
  currentMonday.setDate(selected.getDate() - mondayOffset)
  const start = new Date(currentMonday)
  start.setDate(currentMonday.getDate() - 42)
  const values = new Map(samples.map((sample) => [sample.date, sample.duration]))

  return Array.from({ length: 49 }, (_, index) => {
    const date = new Date(start)
    date.setDate(start.getDate() + index)
    const dateKey = formatDate(date)
    const duration = values.get(dateKey) ?? null
    return {
      date: dateKey,
      weekday: index % 7,
      weekIndex: Math.floor(index / 7),
      duration,
      intensity: intensityFor(duration),
      available: values.has(dateKey),
    }
  })
}
