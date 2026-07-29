import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'
import SettingsPage from '../pages/SettingsPage.vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const originalConsent = { ...store.state.providerConsent }
const originalRecording = store.state.recording
const originalRecordingStatus = store.state.recordingStatus
const originalRecordingMessage = store.state.recordingMessage

afterEach(() => {
  store.state.providerConsent = { ...originalConsent }
  store.state.recording = originalRecording
  store.state.recordingStatus = originalRecordingStatus
  store.state.recordingMessage = originalRecordingMessage
})

describe('settings privacy surface', () => {
  it('keeps one authorization switch without the removed boundary cards', async () => {
    store.state.providerConsent = {
      version: 2,
      noticeSeen: false,
      aiAgentToolsEnabled: false,
    }
    const wrapper = mount(SettingsPage, {
      global: { stubs: { PageHeader: true } },
    })

    expect(wrapper.text()).not.toContain('输入统计与隐私')
    expect(wrapper.text()).not.toContain('启用前请了解读取与上报边界')
    expect(wrapper.text()).not.toContain('接入前历史不会被补造')
    expect(wrapper.findAll('.provider-list input[type="checkbox"]')).toHaveLength(1)

    store.state.providerConsent.noticeSeen = true
    await nextTick()
    const switches = wrapper.findAll('.provider-list input[type="checkbox"]')
    expect(switches).toHaveLength(1)
    expect(wrapper.text()).toContain('匿名上报硬件与工具汇总')
    expect(switches[0].attributes('checked')).toBeUndefined()
    wrapper.unmount()
  })

  it('disables the recording switch while the backend transition is pending', async () => {
    store.state.recordingStatus = 'loading'
    store.state.recordingMessage = '暂停中'
    const wrapper = mount(SettingsPage, {
      global: { stubs: { PageHeader: true } },
    })
    const activityRow = wrapper.findAll('.control-row')
      .find((row) => row.text().includes('活动记录'))
    expect(activityRow?.text()).toContain('暂停中')
    expect(activityRow?.get('input').attributes('disabled')).toBeDefined()

    store.state.recordingStatus = 'ready'
    await nextTick()
    expect(activityRow?.get('input').attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })
})
