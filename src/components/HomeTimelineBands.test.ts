import { describe, expect, it } from 'vitest'
import type { TimelineSegment } from '../domain/events'
import { buildHomeTimelineBands } from './homeTimelineBands'

const hour = 60 * 60 * 1000
const range = { start: 0, end: 24 * hour }

describe('home timeline bands', () => {
  it('creates six consecutive four-hour bands', () => {
    const bands = buildHomeTimelineBands(range, [])
    expect(bands).toHaveLength(6)
    expect(bands.map((band) => band.label)).toEqual([
      '00:00 – 04:00',
      '04:00 – 08:00',
      '08:00 – 12:00',
      '12:00 – 16:00',
      '16:00 – 20:00',
      '20:00 – 24:00',
    ])
    expect(bands[5].range).toEqual({ start: 20 * hour, end: 24 * hour })
  })

  it.each([23, 25])('keeps all bands at four hours when the supplied day spans %i hours', (dayHours) => {
    const bands = buildHomeTimelineBands({ start: 0, end: dayHours * hour }, [])
    expect(bands.every((band) => band.range.end - band.range.start === 4 * hour)).toBe(true)
    expect(bands[5].range).toEqual({ start: 20 * hour, end: 24 * hour })
  })

  it('clips a segment that crosses a band boundary into both bands', () => {
    const segment: TimelineSegment = { start: 3.5 * hour, end: 4.5 * hour, kind: 'attention' }
    const bands = buildHomeTimelineBands(range, [segment])
    expect(bands[0].segments[0]).toMatchObject({ start: 3.5 * hour, end: 4 * hour })
    expect(bands[1].segments[0]).toMatchObject({ start: 4 * hour, end: 4.5 * hour })
  })

  it('keeps empty bands empty', () => {
    const bands = buildHomeTimelineBands(range, [{ start: 8 * hour, end: 9 * hour, kind: 'media' }])
    expect(bands[0].segments).toEqual([])
    expect(bands[2].segments).toHaveLength(1)
  })
})
