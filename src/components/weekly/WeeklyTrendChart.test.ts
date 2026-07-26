import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import WeeklyTrendChart, { type TrendPoint } from './WeeklyTrendChart.vue'

const points: TrendPoint[] = Array.from({ length: 7 }, (_, index) => ({
  label: `周${index + 1}`,
  note: `7/${20 + index}`,
  attention: 2 + index / 2,
  ai: index / 4,
}))

describe('WeeklyTrendChart', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    Reflect.deleteProperty(Element.prototype, 'scrollIntoView')
  })

  it('brings a keyboard-focused trend point into the visible scroll area', async () => {
    const scrollIntoView = vi.fn()
    Object.defineProperty(Element.prototype, 'scrollIntoView', {
      configurable: true,
      value: scrollIntoView,
    })
    const wrapper = mount(WeeklyTrendChart, { props: { points } })
    const firstPoint = wrapper.get('g[role="img"]')

    await firstPoint.trigger('focus')

    expect(scrollIntoView).toHaveBeenCalledWith({
      block: 'nearest',
      inline: 'nearest',
      behavior: 'instant',
    })
    expect(wrapper.get('[role="tooltip"]').text()).toContain('周1 7/20')
  })
})
