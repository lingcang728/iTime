import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SparkLine from './SparkLine.vue'

describe('SparkLine', () => {
  it('renders endpoints and exposes every value to keyboard users', async () => {
    const wrapper = mount(SparkLine, {
      props: {
        values: [12, 28, 21],
        labels: ['周一', '周二', '周三'],
        valueSuffix: ' 小时',
        ariaLabel: '本周活动',
      },
    })

    const points = wrapper.findAll('.spark-point')
    expect(points).toHaveLength(3)
    expect(wrapper.findAll('.spark-point.endpoint')).toHaveLength(2)
    expect(points[1].attributes('aria-label')).toBe('周二，28 小时')

    await points[1].trigger('focus')
    expect(wrapper.get('.spark-tooltip').text()).toBe('周二 · 28 小时')
    await points[1].trigger('blur')
    expect(wrapper.find('.spark-tooltip').exists()).toBe(false)
  })

  it('shows an explicit empty state without interactive points', () => {
    const wrapper = mount(SparkLine, { props: { values: [] } })
    expect(wrapper.findAll('.spark-point')).toHaveLength(0)
    expect(wrapper.text()).toContain('暂无趋势数据')
  })
})
