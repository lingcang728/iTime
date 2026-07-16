import { mount } from '@vue/test-utils'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import AppIcon from './AppIcon.vue'
import { clearAppIconMemory } from '../services/appIconService'

vi.mock('../platform/desktop', () => ({
  isTauriRuntime: () => false,
  listenDesktop: async () => () => undefined,
}))

describe('AppIcon', () => {
  beforeEach(() => {
    clearAppIconMemory()
  })

  it('shows glyph fallback without broken image when no embedded icon', async () => {
    const wrapper = mount(AppIcon, {
      props: { appIdentity: 'unknown-app-xyz', appName: 'Mystery', size: 24 },
    })
    await Promise.resolve()
    await Promise.resolve()
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain('M')
    expect(wrapper.classes().join(' ') + wrapper.find('.app-icon').classes().join(' ')).toMatch(/fallback|unknown|loading|failed/)
  })

  it('uses embedded / local asset for known keys without native bridge', async () => {
    const wrapper = mount(AppIcon, {
      props: { iconKey: 'chrome', appName: 'Chrome', size: 20 },
    })
    await Promise.resolve()
    await Promise.resolve()
    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBeTruthy()
  })

  it('switches to glyph on image error', async () => {
    const wrapper = mount(AppIcon, {
      props: { iconKey: 'chrome', appName: 'Chrome', size: 20 },
    })
    await Promise.resolve()
    const img = wrapper.find('img')
    if (img.exists()) {
      await img.trigger('error')
      expect(wrapper.find('img').exists()).toBe(false)
      expect(wrapper.text()).toContain('C')
    }
  })
})
