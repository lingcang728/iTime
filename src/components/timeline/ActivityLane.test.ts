import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ActivityLane from './ActivityLane.vue'

describe('ActivityLane', () => {
  it('insets adjacent intervals to preserve a visible gap without changing their time labels', () => {
    const start = new Date(2026, 6, 26, 0, 0).getTime()
    const wrapper = mount(ActivityLane, {
      props: {
        label: '前台应用',
        range: { start, end: start + 3_600_000 },
        segments: [
          { start, end: start + 1_800_000, title: '编辑器', color: '#5687b9', kind: 'other' },
          { start: start + 1_800_000, end: start + 3_600_000, title: '浏览器', color: '#5f9a83', kind: 'other' },
        ],
      },
    })

    const segments = wrapper.findAll('.lane-segment')
    expect(segments).toHaveLength(2)
    expect(segments[0].attributes('style')).toContain('--segment-gap: 2px')
    expect(segments[1].attributes('style')).toContain('--segment-left: 50%')
    expect(segments[0].attributes('aria-label')).toContain('00:00 至 00:30')
    expect(segments[1].attributes('aria-label')).toContain('00:30 至 01:00')
  })
})
