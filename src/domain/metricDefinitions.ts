import type { StatUnit } from './events'

export type MetricDefinitionId =
  | 'computerActivity'
  | 'foregroundActivity'
  | 'foregroundFocusRatio'
  | 'foregroundSwitches'
  | 'aiInteraction'
  | 'providerCoverage'
  | 'aiLeverage'
  | 'providerParallelRatio'
  | 'providerConcurrency'
  | 'inputKeyStrokes'

export interface MetricDefinition {
  name: string
  formula: string
  unit: StatUnit
  window: string
  missing: string
}

export const metricDefinitions: Record<MetricDefinitionId, MetricDefinition> = {
  computerActivity: {
    name: '设备活动',
    formula: '设备活跃/空闲时间并集',
    unit: 'milliseconds',
    window: '当日',
    missing: '无数据',
  },
  foregroundActivity: {
    name: '前台专注',
    formula: '设备活跃 ∩ 前台应用',
    unit: 'milliseconds',
    window: '当日',
    missing: '无数据',
  },
  foregroundFocusRatio: {
    name: '专注占比',
    formula: '前台专注 ÷ 设备活动',
    unit: 'ratio',
    window: '当日',
    missing: '无分母',
  },
  foregroundSwitches: {
    name: '应用切换',
    formula: '相邻前台应用变化次数',
    unit: 'count',
    window: '当日',
    missing: '无序列',
  },
  aiInteraction: {
    name: 'AI 前台',
    formula: 'AI 工具前台时间并集',
    unit: 'milliseconds',
    window: '当日',
    missing: '无数据',
  },
  providerCoverage: {
    name: 'Agent 覆盖',
    formula: 'Agent 执行时间并集',
    unit: 'milliseconds',
    window: '当日',
    missing: '无数据',
  },
  aiLeverage: {
    name: 'AI 杠杆',
    formula: 'Agent 执行 ÷ AI 前台',
    unit: 'ratio',
    window: '当日',
    missing: '无分母',
  },
  providerParallelRatio: {
    name: '并行占比',
    formula: '前台与执行重叠 ÷ Agent 覆盖',
    unit: 'ratio',
    window: '当日',
    missing: '无数据',
  },
  providerConcurrency: {
    name: '最高并发',
    formula: '同一时刻最大重叠数',
    unit: 'count',
    window: '当日',
    missing: '无数据',
  },
  inputKeyStrokes: {
    name: '字符键',
    formula: '字符键按下次数（非上屏文字）',
    unit: 'count',
    window: '当日',
    missing: '无计数',
  },
}

export function metricInfo(id: MetricDefinitionId): string {
  const metric = metricDefinitions[id]
  return `${metric.formula} · ${metric.window}`
}

export function comparisonLabel(
  current: number | null,
  previous: number | null,
  formatDelta: (absoluteDelta: number) => string,
  reference = '昨日',
): string {
  if (current === null) return '当前暂无数据'
  if (previous === null) return '暂无对比'
  const delta = current - previous
  if (delta === 0) return `与${reference}持平`
  return `较${reference} ${delta > 0 ? '+' : '−'}${formatDelta(Math.abs(delta))}`
}
