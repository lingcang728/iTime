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

  it('keeps the insight and real-interval language on the AI page', () => {
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
    expect(wrapper.text()).toContain('今日洞察')
    expect(wrapper.text()).toContain('实心执行区间仅来自 Provider')
    expect(wrapper.text()).toContain('没有 Provider 证据时')
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
    expect(wrapper.findAll('.timeline-overview .metric-card')).toHaveLength(3)
    expect(wrapper.get('.full-timeline').classes()).not.toContain('card')
    expect(wrapper.text()).toContain('设备非活跃')
    expect(wrapper.text()).toContain('AI 前台')
  })
})
