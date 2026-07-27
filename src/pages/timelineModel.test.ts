import { describe, expect, it } from 'vitest'
import {
  isSelectedLocalDay,
  timelineNowPercent,
  timelineRange,
  timelineTicks,
} from './timelineModel'

describe('timeline local-day ranges', () => {
  const day = {
    start: new Date(2026, 6, 27, 0, 0, 0, 0).getTime(),
    end: new Date(2026, 6, 28, 0, 0, 0, 0).getTime(),
  }

  it('defaults to the complete local day and offers a real work range', () => {
    expect(timelineRange(day, 'day')).toEqual(day)
    const work = timelineRange(day, 'work')
    expect(new Date(work.start).getHours()).toBe(9)
    expect(new Date(work.end).getHours()).toBe(18)
    expect(timelineTicks('day').map((tick) => tick.label)).toEqual([
      '00:00', '03:00', '06:00', '09:00', '12:00', '15:00', '18:00', '21:00', '24:00',
    ])
  })

  it('compares today in local time instead of UTC', () => {
    const lateLocal = new Date(2026, 6, 27, 23, 59).getTime()
    expect(isSelectedLocalDay('2026-07-27', lateLocal)).toBe(true)
    expect(isSelectedLocalDay('2026-07-28', lateLocal)).toBe(false)
  })

  it('hides the current-time marker outside the selected range', () => {
    const work = timelineRange(day, 'work')
    expect(timelineNowPercent('2026-07-27', work, new Date(2026, 6, 27, 8, 59).getTime())).toBeNull()
    expect(timelineNowPercent('2026-07-27', work, new Date(2026, 6, 27, 18, 0).getTime())).toBeNull()
    expect(timelineNowPercent('2026-07-26', work, new Date(2026, 6, 27, 12, 0).getTime())).toBeNull()
    expect(timelineNowPercent('2026-07-27', work, new Date(2026, 6, 27, 13, 30).getTime())).toBe(50)
  })
})
