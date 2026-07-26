import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import InputTrendChart, { type InputTrendPoint } from './InputTrendChart.vue'

const points: InputTrendPoint[] = [
  { label: '7/20', accessibleLabel: '7月20日', value: 3_000 },
  { label: '7/21', accessibleLabel: '7月21日', value: 3_600 },
  { label: '7/22', accessibleLabel: '7月22日', value: 11_007 },
  { label: '7/23', accessibleLabel: '7月23日', value: 4_800 },
  { label: '7/24', accessibleLabel: '7月24日', value: 900 },
  { label: '7/25', accessibleLabel: '7月25日', value: 5_200 },
  { label: '7/26', accessibleLabel: '7月26日', value: 120 },
]

describe('InputTrendChart', () => {
  it('keeps every bar inside the plot and uses a tighter readable scale', () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'bar', ariaLabel: '七天输入柱状图' },
    })

    expect(wrapper.get('.trend-y-axis span').text()).toBe('15k')
    const bars = wrapper.findAll<SVGRectElement>('.trend-bar')
    expect(bars).toHaveLength(7)
    for (const bar of bars) {
      const x = Number(bar.attributes('x'))
      const width = Number(bar.attributes('width'))
      expect(x).toBeGreaterThanOrEqual(0)
      expect(x + width).toBeLessThanOrEqual(100)
    }
  })

  it('does not remount and replay the line animation for value-only refreshes', async () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'line', ariaLabel: '七天输入折线图' },
    })
    const series = wrapper.get('.trend-series').element
    const pointButtons = wrapper.findAll<HTMLElement>('.trend-point')
    const firstPosition = Number.parseFloat((pointButtons.at(0)?.attributes('style') ?? '').match(/left:\s*([\d.]+)%/)?.[1] ?? '0')
    const lastPosition = Number.parseFloat((pointButtons.at(-1)?.attributes('style') ?? '').match(/left:\s*([\d.]+)%/)?.[1] ?? '100')

    expect(firstPosition).toBeGreaterThan(0)
    expect(lastPosition).toBeLessThan(100)

    await wrapper.setProps({
      points: points.map((point, index) => index === 6 ? { ...point, value: point.value + 40 } : point),
    })
    expect(wrapper.get('.trend-series').element).toBe(series)
  })
})
