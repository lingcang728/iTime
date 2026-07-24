import { describe, expect, it } from 'vitest'
import { isWithinQuietHours, reminderOccurrenceKey, shouldShowRestReminder } from './reminders'

describe('rest reminder timing', () => {
  it('supports quiet hours that cross midnight', () => {
    expect(isWithinQuietHours(new Date('2026-07-17T23:00:00'), '22:00', '08:00')).toBe(true)
    expect(isWithinQuietHours(new Date('2026-07-17T07:30:00'), '22:00', '08:00')).toBe(true)
    expect(isWithinQuietHours(new Date('2026-07-17T12:00:00'), '22:00', '08:00')).toBe(false)
  })

  it('suppresses stale and quiet-hour reminders', () => {
    const now = new Date('2026-07-17T12:00:00')
    const base = {
      enabled: true,
      continuousDuration: 60_000,
      targetDuration: 60_000,
      lastActiveEnd: now.getTime() - 10_000,
      now,
      quietStart: '22:00',
      quietEnd: '08:00',
      dismissed: false,
    }
    expect(shouldShowRestReminder(base)).toBe(true)
    expect(shouldShowRestReminder({ ...base, lastActiveEnd: now.getTime() - 61_000 })).toBe(false)
    expect(shouldShowRestReminder({ ...base, quietStart: '11:00', quietEnd: '13:00' })).toBe(false)
  })

  it('gives each recurring interval its own dismissal key', () => {
    expect(reminderOccurrenceKey('2026-07-24', 29 * 60_000, 30 * 60_000)).toBeNull()
    expect(reminderOccurrenceKey('2026-07-24', 30 * 60_000, 30 * 60_000))
      .toBe('2026-07-24:1800000:1')
    expect(reminderOccurrenceKey('2026-07-24', 60 * 60_000, 30 * 60_000))
      .toBe('2026-07-24:1800000:2')
  })
})
