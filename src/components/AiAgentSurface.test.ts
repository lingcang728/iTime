import { mount } from '@vue/test-utils'
import { PhRobot } from '@phosphor-icons/vue'
import { markRaw } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'
import AiAgentsPage from '../pages/AiAgentsPage.vue'
import TimelinePage from '../pages/TimelinePage.vue'
import { useAppStore } from '../stores/appStore'
import AiDetailDrawer from './AiDetailDrawer.vue'
import MetricCard from './MetricCard.vue'

const store = useAppStore()

afterEach(() => store.closeTool())

describe('AI agent surface', () => {
  it('renders a consistent vector metric icon and a plain-language definition', () => {
    const wrapper = mount(MetricCard, {
      props: {
        label: '有效执行',
        value: '2 小时 36 分',
        detail: '各工具执行区间分别累计',
        icon: markRaw(PhRobot),
        tone: 'accent',
        info: '当前数据源归为 AI 工作的区间总和。',
      },
    })
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.find('svg').exists()).toBe(true)
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

  it('keeps Provider and foreground evidence clearly separated on the AI page', () => {
    const wrapper = mount(AiAgentsPage, {
      global: {
        stubs: {
          PageHeader: true,
          MetricCard: true,
          AiActivityTimeline: true,
          ApplicationIcon: true,
        },
      },
    })
    expect(wrapper.text()).toContain('AI 证据时间线')
    expect(wrapper.text()).toContain('Provider 证据来自 Codex / Claude Code 本机会话时间事件')
    expect(wrapper.text()).toContain('不读取会话内容')
    expect(wrapper.text()).toContain('真实执行区间协作图')
    expect(wrapper.text()).not.toContain('节省时间')
    expect(wrapper.text()).not.toContain('AI 产出')
  })

  it('uses one flat evidence surface and a neutral timeline taxonomy', () => {
    const wrapper = mount(TimelinePage, {
      global: {
        stubs: {
          PageHeader: true,
          ActivityLane: true,
          Transition: false,
        },
      },
    })
    expect(wrapper.findAll('.timeline-overview .metric-card')).toHaveLength(4)
    expect(wrapper.get('.full-timeline').classes()).not.toContain('card')
    expect(wrapper.text()).toContain('设备非活跃')
    expect(wrapper.text()).toContain('AI 前台')
    expect(wrapper.text()).toContain('Provider')
  })
})
