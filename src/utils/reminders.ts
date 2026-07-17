export function isWithinQuietHours(now: Date, start: string, end: string): boolean {
  const current = now.getHours() * 60 + now.getMinutes()
  const [startHour = 0, startMinute = 0] = start.split(':').map(Number)
  const [endHour = 0, endMinute = 0] = end.split(':').map(Number)
  const startMinutes = startHour * 60 + startMinute
  const endMinutes = endHour * 60 + endMinute
  return startMinutes === endMinutes
    || (startMinutes < endMinutes
      ? current >= startMinutes && current < endMinutes
      : current >= startMinutes || current < endMinutes)
}

export function shouldShowRestReminder(input: {
  enabled: boolean
  continuousDuration: number
  targetDuration: number
  lastActiveEnd: number | null
  now: Date
  quietStart: string
  quietEnd: string
  dismissed: boolean
}): boolean {
  return input.enabled
    && input.continuousDuration >= input.targetDuration
    && input.lastActiveEnd !== null
    && input.now.getTime() - input.lastActiveEnd < 60_000
    && !isWithinQuietHours(input.now, input.quietStart, input.quietEnd)
    && !input.dismissed
}
