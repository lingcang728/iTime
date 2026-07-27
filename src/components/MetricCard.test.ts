import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import MetricCard from './MetricCard.vue'

describe('MetricCard', () => {
  it('renders a progress ring only when a real bounded progress value exists', () => {
    const missing = mount(MetricCard, {
      props: { label: '比率', value: '—', detail: '暂无数据', visual: 'ring' },
    })
    expect(missing.find('.metric-ring').exists()).toBe(false)

    const available = mount(MetricCard, {
      props: { label: '比率', value: '25%', detail: '真实分母', visual: 'ring', progress: 0.25 },
    })
    expect(available.get('.metric-ring__value').attributes('style')).toContain('80.25')
  })

  it('does not invent decorative trend bars when no series is supplied', () => {
    const wrapper = mount(MetricCard, {
      props: { label: '计数', value: '3', detail: '暂无趋势', visual: 'bars' },
    })
    expect(wrapper.find('.metric-bars').exists()).toBe(false)
  })

  it('opens the circular info definition with fixed viewport placement', async () => {
    const wrapper = mount(MetricCard, {
      props: {
        label: '前台专注时长',
        value: '4.2 小时',
        detail: '较昨日 -7 小时',
        info: '设备活跃区间与前台应用区间的交集；单位：milliseconds；窗口：所选本地自然日。',
      },
      attachTo: document.body,
    })

    const button = wrapper.get('.metric-info')
    await button.trigger('focus')
    const tooltip = wrapper.get('[role="tooltip"]')
    expect(tooltip.classes()).toContain('is-open')
    expect(wrapper.classes()).toContain('metric-card--tooltip-open')
    expect(tooltip.attributes('style')).toMatch(/position:\s*fixed|top:|left:|width:/)
    expect(tooltip.text()).toContain('设备活跃区间')
    await button.trigger('blur')
    expect(tooltip.classes()).not.toContain('is-open')
    wrapper.unmount()
  })
})
