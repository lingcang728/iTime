import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import AiAgentsPage from '../pages/AiAgentsPage.vue'
import { useAppStore } from '../stores/appStore'
import AgentMetricCard from './AgentMetricCard.vue'
import AiDetailDrawer from './AiDetailDrawer.vue'

const store = useAppStore()

afterEach(() => store.closeTool())

describe('AI agent surface', () => {
  it('renders the generated metric artwork and a plain-language definition', () => {
    const wrapper = mount(AgentMetricCard, {
      props: {
        label: '有效执行',
        value: '2 小时 36 分',
        detail: '各工具执行区间分别累计',
        iconSrc: '/metric.png',
        tone: 'violet',
        info: '当前数据源归为 AI 工作的区间总和。',
      },
    })
    expect(wrapper.get('img').attributes('src')).toBe('/metric.png')
    expect(wrapper.get('[role="tooltip"]').text()).toContain('区间总和')
  })

  it('explains estimates, privacy limits and confidence in tool details', () => {
    store.openTool('codex')
    const wrapper = mount(AiDetailDrawer, {
      global: { stubs: { ApplicationIcon: true, Transition: false } },
    })
    expect(wrapper.text()).toContain('前台活跃')
    expect(wrapper.text()).toContain('不读取提示词、对话、按键内容或文件正文')
    expect(wrapper.text()).toContain('不是工具的“知性度”')
    expect(wrapper.text()).toContain('检测置信度')
    wrapper.unmount()
  })

  it('keeps the insight and real-interval language on the AI page', () => {
    const wrapper = mount(AiAgentsPage, {
      global: {
        stubs: {
          PageHeader: true,
          AgentMetricCard: true,
          AiActivityTimeline: true,
          ApplicationIcon: true,
        },
      },
    })
    expect(wrapper.text()).toContain('今日洞察')
    expect(wrapper.text()).toContain('实心执行区间仅来自 Provider')
    expect(wrapper.text()).toContain('没有 Provider 证据时')
  })
})
