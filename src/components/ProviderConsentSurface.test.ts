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

describe('AI Agent tool consent surface', () => {
  it('shows the complete boundary and exactly one authorization switch', async () => {
    store.state.providerConsent = {
      version: 2,
      noticeSeen: false,
      aiAgentToolsEnabled: false,
    }
    const wrapper = mount(SettingsPage, {
      global: { stubs: { PageHeader: true } },
    })

    expect(wrapper.text()).toContain('启用前请了解读取与上报边界')
    expect(wrapper.text()).toContain('不会读取、保存、哈希或上传提示词')
    expect(wrapper.text()).toContain('Cursor、Antigravity、Codex、Claude Code、OpenCode、Grok Build、Hermes 与 OpenClaw')
    expect(wrapper.findAll('.provider-list input[type="checkbox"]')).toHaveLength(1)

    store.state.providerConsent.noticeSeen = true
    await nextTick()
    const switches = wrapper.findAll('.provider-list input[type="checkbox"]')
    expect(switches).toHaveLength(1)
    expect(wrapper.text()).toContain('匿名硬件、性能与每日工具汇总')
    expect(switches[0].attributes('checked')).toBeUndefined()
    wrapper.unmount()
  })

  it('disables the recording switch while the backend transition is pending', async () => {
    store.state.recordingStatus = 'loading'
    store.state.recordingMessage = '正在暂停并刷新当前片段'
    const wrapper = mount(SettingsPage, {
      global: { stubs: { PageHeader: true } },
    })
    const activityRow = wrapper.findAll('.control-row')
      .find((row) => row.text().includes('活动记录'))
    expect(activityRow?.text()).toContain('正在暂停并刷新当前片段')
    expect(activityRow?.get('input').attributes('disabled')).toBeDefined()

    store.state.recordingStatus = 'ready'
    await nextTick()
    expect(activityRow?.get('input').attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })
})
