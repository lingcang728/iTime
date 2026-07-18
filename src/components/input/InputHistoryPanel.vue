<script setup lang="ts">
import { computed } from 'vue'
import { PhCalendarBlank, PhChartLineUp, PhShieldCheck } from '@phosphor-icons/vue'
import SparkLine from '../SparkLine.vue'
import type { InputActivityPoint, InputGranularity } from '../../providers/inputActivity'
import { formatNumber } from '../../utils/format'

const props = defineProps<{
  history: InputActivityPoint[]
  granularity: InputGranularity | 'none'
}>()

const rawPoints = computed(() => [...props.history].sort((first, second) => first.start - second.start))
const points = computed(() => {
  if (props.granularity !== 'minute') return rawPoints.value
  const groups = new Map<number, number>()
  for (const point of rawPoints.value) {
    const date = new Date(point.start)
    const start = new Date(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours()).getTime()
    groups.set(start, (groups.get(start) ?? 0) + point.keyStrokes)
  }
  return [...groups.entries()].sort(([left], [right]) => left - right).map(([start, keyStrokes]) => ({
    start,
    end: start + 3_600_000,
    keyStrokes,
    leftClicks: null,
    rightClicks: null,
    combinedClicks: 0,
    mouseDistance: 0,
    scrollDistance: 0,
  }))
})
const hasSeries = computed(() => points.value.length > 1)
const values = computed(() => points.value.map((point) => point.keyStrokes))
const labels = computed(() => points.value.map((point) => pointLabel(point.start)))
const title = computed(() => props.granularity === 'day' ? '每日输入趋势' : '今日输入趋势')
const subtitle = computed(() => props.granularity === 'day'
  ? '按天汇总，不推断小时内的输入分布'
  : props.granularity === 'minute'
    ? '分钟级采集，趋势按小时汇总；悬停或聚焦查看数值'
    : `${granularityLabel.value}聚合；悬停或聚焦查看数值`)
const granularityLabel = computed(() => ({ minute: '分钟', hour: '小时', day: '每日', none: '无' })[props.granularity])
const singlePoint = computed(() => points.value[0] ?? null)

function pointLabel(timestamp: number): string {
  const date = new Date(timestamp)
  if (props.granularity === 'day') return `${date.getMonth() + 1}月${date.getDate()}日`
  const hour = String(date.getHours()).padStart(2, '0')
  const minute = String(date.getMinutes()).padStart(2, '0')
  return props.granularity === 'hour' || props.granularity === 'minute' ? `${hour}:00` : `${hour}:${minute}`
}

</script>

<template>
  <section class="input-history-panel">
    <template v-if="hasSeries">
      <section class="trend-section">
        <header>
          <div><h2>{{ title }}</h2><p>{{ subtitle }}</p></div>
          <span class="privacy-mark"><PhShieldCheck :size="15" weight="regular" />聚合数据</span>
        </header>
        <div class="trend-chart">
          <SparkLine
            :values="values"
            :labels="labels"
            value-suffix=" 次"
            color="var(--accent-green)"
            :aria-label="title"
          />
        </div>
        <div class="trend-labels"><span v-for="label in labels" :key="label">{{ label }}</span></div>
      </section>

    </template>

    <section v-else-if="singlePoint" class="single-day">
      <span class="single-day__icon"><PhCalendarBlank :size="22" weight="regular" /></span>
      <div>
        <span>所选日期输入汇总</span>
        <strong>{{ formatNumber(singlePoint.keyStrokes) }}<small> 个字符键</small></strong>
        <p v-if="granularity === 'day'">数据源仅提供日级总量，因此不生成小时趋势或连续输入结论。</p>
        <p v-else>当前只有一个聚合点，记录更多数据后才会形成趋势。</p>
      </div>
      <dl>
        <div><dt>计数方式</dt><dd>字符键按下</dd></div>
        <div><dt>记录粒度</dt><dd>{{ granularityLabel }}</dd></div>
      </dl>
    </section>

    <section v-else class="history-empty">
      <PhChartLineUp :size="24" weight="regular" />
      <div><strong>这一天没有输入汇总</strong><p>切换到有记录的日期后，这里会显示数据源实际提供的粒度。</p></div>
    </section>
  </section>
</template>

<style scoped>
.input-history-panel {
  min-width: 0;
  display: grid;
  grid-template-columns: 1fr;
  gap: 0;
  padding-block: 20px;
  border-block: 1px solid var(--border-soft);
}

.input-history-panel:has(.single-day),
.input-history-panel:has(.history-empty) { grid-template-columns: 1fr; }

header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
h2 { margin: 0; color: var(--text-primary); font-size: 14px; font-weight: 650; letter-spacing: -.2px; }
p { margin: 4px 0 0; color: var(--text-secondary); font-size: 10px; line-height: 1.55; }
.privacy-mark { display: inline-flex; align-items: center; gap: 5px; flex: 0 0 auto; padding-top: 2px; color: var(--accent-green-strong); font-size: 10px; font-weight: 600; }
.trend-section { min-width: 0; }
.trend-chart { height: 126px; margin-top: 14px; padding: 6px 4px 0; border-bottom: 1px solid var(--border-strong); background-image: linear-gradient(to bottom, transparent 32%, var(--border-soft) 33%, transparent 34%, transparent 65%, var(--border-soft) 66%, transparent 67%); }
.trend-labels { display: flex; justify-content: space-between; gap: 6px; margin-top: 8px; color: var(--text-muted); font-size: 10px; font-variant-numeric: tabular-nums; }
.single-day { min-height: 112px; display: grid; grid-template-columns: auto minmax(0, 1.35fr) minmax(250px, .75fr); align-items: center; gap: 16px; }
.single-day__icon { width: 38px; height: 38px; display: grid; place-items: center; color: var(--accent-green-strong); }
.single-day > div > span { color: var(--text-secondary); font-size: 10px; }
.single-day strong { display: block; margin-top: 3px; color: var(--text-primary); font: 650 25px/1.15 var(--font-data); font-variant-numeric: tabular-nums; letter-spacing: -.7px; }
.single-day strong small { color: var(--text-secondary); font-size: 11px; font-weight: 600; letter-spacing: 0; }
dl { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 14px; margin: 0; padding: 12px 14px; border: 1px solid var(--border-soft); border-radius: 8px; background: var(--bg-inset); }
dl div { display: flex; justify-content: space-between; gap: 12px; }
dt { color: var(--text-secondary); font-size: 10px; }
dd { margin: 0; color: var(--text-primary); font: 650 11px/1.2 var(--font-data); font-variant-numeric: tabular-nums; }
.history-empty { min-height: 112px; display: flex; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); }
.history-empty strong { color: var(--text-primary); font-size: 12px; }

@media (max-width: 920px) {
  .input-history-panel { grid-template-columns: 1fr; }
}

@media (max-width: 760px) {
  .single-day { grid-template-columns: auto 1fr; }
  dl { grid-column: 1 / -1; }
}
</style>
