import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppMark from './AppMark.vue'

describe('AppMark', () => {
  it('renders the original green clock logo at the requested size', () => {
    const wrapper = mount(AppMark, { props: { size: 34 } })
    const mark = wrapper.get('.app-mark')
    const image = wrapper.get('img')

    expect(mark.attributes('style')).toContain('width: 34px')
    expect(mark.attributes('style')).toContain('height: 34px')
    expect(image.attributes('src')).toMatch(/^data:image\/svg\+xml/)
    expect(image.attributes('src')).toContain('%2368c98b')
    expect(image.attributes('src')).toContain('%236658d7')
    expect(image.attributes('alt')).toBe('')
    expect(image.attributes('draggable')).toBe('false')
  })
})
