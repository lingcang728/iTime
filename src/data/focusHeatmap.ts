export interface HeatmapDay {
  date: string
  weekday: number
  weekIndex: number
  duration: number
  intensity: number
}

const hour = 3_600_000
const fixedHours = [
  2.1, 4.2, 3.6, 5.1, 4.8, 1.4, 0,
  3.4, 5.7, 4.6, 6.2, 5.3, 2.7, 1.2,
  4.1, 6.4, 5.9, 7.1, 6.8, 3.2, 1.6,
  5.2, 6.9, 7.4, 5.8, 6.1, 2.4, 0.8,
  4.7, 7.2, 6.3, 7.8, 5.6, 3.9, 1.1,
  6.1, 7.2, 8.6, 5.3, 7.8, 6.5, 6.8,
  7.8, 6.5, 6.8, 0, 0, 0, 0,
]

function formatDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function intensityFor(hours: number): number {
  if (!hours) return 0
  if (hours < 2) return 1
  if (hours < 4) return 2
  if (hours < 6) return 3
  if (hours < 7.5) return 4
  return 5
}

export function buildFocusHeatmap(endDate: string): HeatmapDay[] {
  const selected = new Date(`${endDate}T12:00:00`)
  const mondayOffset = (selected.getDay() + 6) % 7
  const currentMonday = new Date(selected)
  currentMonday.setDate(selected.getDate() - mondayOffset)
  const start = new Date(currentMonday)
  start.setDate(currentMonday.getDate() - 42)

  return fixedHours.map((hours, index) => {
    const date = new Date(start)
    date.setDate(start.getDate() + index)
    const future = date.getTime() > selected.getTime()
    const durationHours = future ? 0 : hours
    return {
      date: formatDate(date),
      weekday: index % 7,
      weekIndex: Math.floor(index / 7),
      duration: durationHours * hour,
      intensity: intensityFor(durationHours),
    }
  })
}
