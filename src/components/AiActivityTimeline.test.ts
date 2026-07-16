import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { AiToolSummary, ForegroundAppInterval } from '../domain/events'
import AiActivityTimeline from './AiActivityTimeline.vue'

const start = new Date('2026-05-20T00:00:00').getTime()
const hour = 3_600_000
const evidence = {
  source: 'test-source',
  accuracyLabel: 'estimated' as const,
  basis: '前台进程与任务事件匹配',
  confidence: .82,
  reviewState: 'needsReview' as const,
}

const foreground: ForegroundAppInterval[] = [{
  ...evidence,
  id: 'foreground-1',
  type: 'foreground',
  start: start + 9 * hour,
  end: start + 11 * hour,
  appId: 'editor',
  appName: 'Editor',
  category: '开发',
  color: '#4f7ee8',
}]

const tools: AiToolSummary[] = [{
  toolId: 'codex',
  toolName: 'Codex',
  status: 'completed',
  iconKey: 'codex',
  foregroundDuration: 20 * 60_000,
  effectiveDuration: hour,
  silentWaitDuration: 15 * 60_000,
  parallelOverlapDuration: 30 * 60_000,
  taskCount: 1,
  contribution: 1,
  accuracyLabel: 'estimated',
  confidence: .82,
  reviewState: 'needsReview',
  workIntervals: [{
    ...evidence,
    id: 'work-1',
    type: 'aiWork',
    start: start + 10 * hour,
    end: start + 11 * hour,
    agentId: 'agent-1',
    toolId: 'codex',
    toolName: 'Codex',
    taskId: 'task-1',
  }],
  interactionIntervals: [{
    ...evidence,
    id: 'interaction-1',
    type: 'aiInteraction',
    start: start + 10 * hour,
    end: start + 10 * hour + 20 * 60_000,
    toolId: 'codex',
    toolName: 'Codex',
  }],
  waitIntervals: [{ start: start + 11 * hour, end: start + 11 * hour + 15 * 60_000 }],
}]

describe('AiActivityTimeline', () => {
  it('renders sampled intervals as focusable segments with exact time labels', () => {
    const wrapper = mount(AiActivityTimeline, {
      props: { range: { start, end: start + 24 * hour }, foreground, tools },
    })
    const segments = wrapper.findAll('button.timeline__segment')
    expect(segments.length).toBeGreaterThanOrEqual(5)
    expect(segments.every((item) => item.attributes('aria-label'))).toBe(true)
    expect(wrapper.text()).toContain('Codex')
    expect(wrapper.text()).toContain('前台交互（估算）')
    expect(wrapper.find('.is-overlap').exists()).toBe(true)
  })

  it('states when no AI tool intervals are available', () => {
    const wrapper = mount(AiActivityTimeline, {
      props: { range: { start, end: start + 24 * hour }, foreground: [], tools: [] },
    })
    expect(wrapper.text()).toContain('尚未采集到 AI 工具执行区间')
  })
})
