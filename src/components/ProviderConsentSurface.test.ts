import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'
import SettingsPage from '../pages/SettingsPage.vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const originalConsent = { ...store.state.providerConsent }

afterEach(() => {
  store.state.providerConsent = { ...originalConsent }
})

describe('Provider consent surface', () => {
  it('shows the one-time boundary before exposing independent source switches', async () => {
    store.state.providerConsent = {
      version: 1,
      noticeSeen: false,
      codexEnabled: false,
      claudeEnabled: false,
    }
    const wrapper = mount(SettingsPage, {
      global: { stubs: { PageHeader: true } },
    })

    expect(wrapper.text()).toContain('先了解读取边界，再选择是否启用')
    expect(wrapper.text()).toContain('不会访问、保存或显示消息正文')
    expect(wrapper.findAll('.provider-list input[type="checkbox"]')).toHaveLength(0)

    store.state.providerConsent.noticeSeen = true
    await nextTick()
    const switches = wrapper.findAll('.provider-list input[type="checkbox"]')
    expect(switches).toHaveLength(2)
    expect(wrapper.text()).toContain('%USERPROFILE%\\.codex\\sessions')
    expect(wrapper.text()).toContain('%USERPROFILE%\\.claude\\projects')
    expect(switches[0].attributes('checked')).toBeUndefined()
    expect(switches[1].attributes('checked')).toBeUndefined()
    wrapper.unmount()
  })
})
