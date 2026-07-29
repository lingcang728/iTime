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
    name: '设备活动时长',
    formula: '设备处于活跃或空闲状态的自然时间并集',
    unit: 'milliseconds',
    window: '所选本地自然日',
    missing: '暂无设备状态数据',
  },
  foregroundActivity: {
    name: '前台专注时长',
    formula: '设备活跃区间与前台应用区间的交集',
    unit: 'milliseconds',
    window: '所选本地自然日',
    missing: '暂无前台应用数据',
  },
  foregroundFocusRatio: {
    name: '前台专注占比',
    formula: '前台专注时长 ÷ 设备活动时长',
    unit: 'ratio',
    window: '所选本地自然日',
    missing: '缺少可用分母',
  },
  foregroundSwitches: {
    name: '前台应用切换',
    formula: '按开始时间排列后，相邻前台应用身份从 A 变为 B 的次数；相邻同应用片段不重复计数',
    unit: 'count',
    window: '所选本地自然日',
    missing: '暂无前台应用序列',
  },
  aiInteraction: {
    name: 'AI 前台活跃',
    formula: 'AI 工具处于前台的自然时间并集',
    unit: 'milliseconds',
    window: '所选本地自然日',
    missing: '暂无 AI 前台活动数据',
  },
  providerCoverage: {
    name: 'AI Agent 执行覆盖',
    formula: '获授权 AI Agent 执行区间的自然时间并集',
    unit: 'milliseconds',
    window: '所选本地自然日',
    missing: '暂无获授权 AI Agent 数据',
  },
  aiLeverage: {
    name: 'AI 杠杆率',
    formula: 'AI Agent 累计执行时长 ÷ AI 前台活跃时长',
    unit: 'ratio',
    window: '所选本地自然日',
    missing: '缺少 AI Agent 执行或 AI 前台活跃分母',
  },
  providerParallelRatio: {
    name: 'AI Agent 并行占比',
    formula: '前台专注与 AI Agent 执行重叠时长 ÷ AI Agent 执行覆盖时长',
    unit: 'ratio',
    window: '所选本地自然日',
    missing: '缺少 AI Agent 覆盖或前台专注数据',
  },
  providerConcurrency: {
    name: 'AI Agent 最高并发',
    formula: '获授权 AI Agent 执行区间在同一时刻的最大重叠数量',
    unit: 'count',
    window: '所选本地自然日',
    missing: '暂无获授权 AI Agent 数据',
  },
  inputKeyStrokes: {
    name: '字符键按下次数',
    formula: 'Windows 键盘 hook 识别到的字符键按下事件计数；不代表最终上屏文字',
    unit: 'count',
    window: '所选本地自然日',
    missing: '暂无字符键计数',
  },
}

export function metricInfo(id: MetricDefinitionId): string {
  const metric = metricDefinitions[id]
  return `${metric.formula}；单位：${metric.unit}；窗口：${metric.window}；缺失状态：${metric.missing}。`
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
