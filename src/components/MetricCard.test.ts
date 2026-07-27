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
})
