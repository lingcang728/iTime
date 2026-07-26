import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import InputHistoryPanel from './InputHistoryPanel.vue'
import type { InputActivityPoint } from '../../providers/inputActivity'

function point(date: string, keyStrokes: number): InputActivityPoint {
  const start = new Date(`${date}T00:00:00`).getTime()
  return {
    start,
    end: start + 86_400_000,
    keyStrokes,
    leftClicks: null,
    rightClicks: null,
    combinedClicks: 0,
    mouseDistance: 0,
    scrollDistance: 0,
  }
}

describe('InputHistoryPanel', () => {
  it('renders a real seven-day history and switches smoothly to bars and thirty days', async () => {
    const wrapper = mount(InputHistoryPanel, {
      props: {
        history: [
          point('2026-07-24', 320),
          point('2026-07-25', 480),
          point('2026-07-26', 200),
        ],
        granularity: 'minute',
        endDate: '2026-07-26',
      },
    })

    expect(wrapper.findAll('.trend-point')).toHaveLength(7)
    expect(wrapper.text()).toContain('1,000')
    expect(wrapper.get('button[aria-pressed="true"]').text()).toContain('折线')

    const barButton = wrapper.findAll('button').find((button) => button.text().includes('柱状'))
    await barButton?.trigger('click')
    expect(wrapper.findAll('.trend-bar')).toHaveLength(7)
    expect(barButton?.attributes('aria-pressed')).toBe('true')

    const monthButton = wrapper.findAll('button').find((button) => button.text().includes('30 天'))
    await monthButton?.trigger('click')
    expect(wrapper.findAll('.trend-point')).toHaveLength(30)
    expect(monthButton?.attributes('aria-pressed')).toBe('true')
  })

  it('keeps an explicit empty state when no keyboard history exists', () => {
    const wrapper = mount(InputHistoryPanel, {
      props: {
        history: [],
        granularity: 'minute',
        endDate: '2026-07-26',
      },
    })

    expect(wrapper.text()).toContain('这一天没有输入汇总')
    expect(wrapper.find('.input-trend-chart').exists()).toBe(false)
  })
})
