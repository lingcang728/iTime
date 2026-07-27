import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
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
  afterEach(() => {
    vi.useRealTimers()
  })

  it('keeps every bar inside the plot and shows values without hover', () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'bar', ariaLabel: '七天输入柱状图' },
    })

    expect(wrapper.get('.trend-y-axis span').text()).toBe('15k')
    const barNodes = wrapper.findAll<HTMLElement>('.trend-bar-node')
    expect(barNodes).toHaveLength(7)
    for (const node of barNodes) {
      const style = node.attributes('style') ?? ''
      const x = Number(style.match(/translate3d\(([\d.]+)%/)?.[1] ?? Number.NaN)
      const scaleX = Number(style.match(/--bar-scale-x:\s*([\d.]+)/)?.[1] ?? Number.NaN)
      const visualWidth = 4.8 * scaleX
      expect(x - visualWidth / 2).toBeGreaterThanOrEqual(0)
      expect(x + visualWidth / 2).toBeLessThanOrEqual(100)
    }
    expect(wrapper.findAll('.trend-bar-value').map((label) => label.text())).toEqual([
      '3,000',
      '3,600',
      '11,007',
      '4,800',
      '900',
      '5,200',
      '120',
    ])
  })

  it('does not remount and replay the line animation for value-only refreshes', async () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'line', ariaLabel: '七天输入折线图' },
    })
    const lineLayer = wrapper.get('.trend-line-layer').element
    const pointNodes = wrapper.findAll<HTMLElement>('.trend-hit-node')
    const firstPosition = Number.parseFloat((pointNodes.at(0)?.attributes('style') ?? '').match(/translate3d\(([\d.]+)%/)?.[1] ?? '0')
    const lastPosition = Number.parseFloat((pointNodes.at(-1)?.attributes('style') ?? '').match(/translate3d\(([\d.]+)%/)?.[1] ?? '100')

    expect(firstPosition).toBeGreaterThan(0)
    expect(lastPosition).toBeLessThan(100)

    await wrapper.setProps({
      points: points.map((point, index) => index === 6 ? { ...point, value: point.value + 40 } : point),
    })
    expect(wrapper.get('.trend-line-layer').element).toBe(lineLayer)
  })

  it('preserves shared dates and identifies both range animation directions', async () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'line', ariaLabel: '七天输入折线图' },
    })
    const finalDateButton = wrapper.get('button[aria-label^="7月26日"]').element
    const earlierPoints = Array.from({ length: 23 }, (_, index): InputTrendPoint => ({
      label: `6/${27 + index}`,
      accessibleLabel: `较早日期${index}`,
      value: index % 4 === 0 ? 800 + index * 70 : 0,
    }))

    await wrapper.setProps({ points: [...earlierPoints, ...points] })
    expect(wrapper.attributes('data-range-motion')).toBe('expanding')
    expect(wrapper.findAll('.trend-hit-node')).toHaveLength(30)
    expect(wrapper.get('button[aria-label^="7月26日"]').element).toBe(finalDateButton)

    await wrapper.setProps({ points })
    expect(wrapper.attributes('data-range-motion')).toBe('contracting')
    expect(wrapper.findAll('.trend-hit-node')).toHaveLength(7)
  })

  it('uses full-height column hits in bar mode without line-style markers', async () => {
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'bar', ariaLabel: '七天输入柱状图' },
    })

    expect(wrapper.classes()).toContain('is-bar')
    expect(wrapper.findAll('.trend-hit-node.is-column')).toHaveLength(7)
    expect(wrapper.findAll('.trend-point.is-marker-visible')).toHaveLength(0)

    const shortBarButton = wrapper.get('button[aria-label^="7月24日"]')
    await shortBarButton.trigger('pointerenter')
    expect(wrapper.find('.trend-bar-node.is-active').exists()).toBe(true)
    expect(wrapper.find('.trend-tooltip.is-visible').text()).toContain('7月24日')
    expect(wrapper.find('.trend-tooltip.is-visible').text()).toContain('900')
  })

  it('keeps the deliberate mode-motion state for the full visual transition', async () => {
    vi.useFakeTimers()
    const wrapper = mount(InputTrendChart, {
      props: { points, mode: 'line', ariaLabel: '七天输入折线图' },
    })

    await wrapper.setProps({ mode: 'bar' })
    expect(wrapper.classes()).toContain('is-mode-motion')

    await vi.advanceTimersByTimeAsync(619)
    expect(wrapper.classes()).toContain('is-mode-motion')

    await vi.advanceTimersByTimeAsync(1)
    expect(wrapper.classes()).not.toContain('is-mode-motion')
  })

  it('uses roving focus and supports arrows, Home, End, Enter and Space', async () => {
    const wrapper = mount(InputTrendChart, {
      attachTo: document.body,
      props: { points, mode: 'line', ariaLabel: '七天输入折线图' },
    })
    const buttons = () => wrapper.findAll<HTMLButtonElement>('.trend-point')
    expect(buttons().filter((button) => button.attributes('tabindex') === '0')).toHaveLength(1)
    expect(buttons().at(-1)?.attributes('tabindex')).toBe('0')

    await buttons().at(-1)?.trigger('keydown', { key: 'Home' })
    expect(buttons()[0].attributes('tabindex')).toBe('0')
    expect(document.activeElement).toBe(buttons()[0].element)

    await buttons()[0].trigger('keydown', { key: 'ArrowRight' })
    expect(document.activeElement).toBe(buttons()[1].element)
    await buttons()[1].trigger('keydown', { key: 'End' })
    expect(document.activeElement).toBe(buttons().at(-1)?.element)
    await buttons().at(-1)?.trigger('keydown', { key: 'Enter' })
    expect(buttons().at(-1)?.attributes('aria-pressed')).toBe('true')
    await buttons()[0].trigger('keydown', { key: ' ' })
    expect(buttons()[0].attributes('aria-pressed')).toBe('true')

    expect(wrapper.get('table caption').text()).toContain('可读数据表')
    expect(wrapper.findAll('tbody tr')).toHaveLength(points.length)
    wrapper.unmount()
  })
})
