<script setup lang="ts">
import { computed, ref } from 'vue'
import { PhChartBar, PhChartLine, PhChartLineUp, PhShieldCheck } from '@phosphor-icons/vue'
import InputTrendChart, { type InputTrendPoint } from './InputTrendChart.vue'
import type { InputActivityPoint, InputGranularity } from '../../providers/inputActivity'
import { formatNumber } from '../../utils/format'

const props = defineProps<{
  history: InputActivityPoint[]
  granularity: InputGranularity | 'none'
  endDate: string
}>()

const rangeDays = ref<7 | 30>(7)
const chartMode = ref<'line' | 'bar'>('line')
const dailyTotals = computed(() => {
  const totals = new Map<string, number>()
  for (const point of props.history) {
    const key = dateKey(new Date(point.start))
    totals.set(key, (totals.get(key) ?? 0) + point.keyStrokes)
  }
  return totals
})
const points = computed<InputTrendPoint[]>(() => {
  const end = new Date(`${props.endDate}T12:00:00`)
  return Array.from({ length: rangeDays.value }, (_, index) => {
    const date = new Date(end)
    date.setDate(end.getDate() - (rangeDays.value - 1 - index))
    const value = dailyTotals.value.get(dateKey(date)) ?? 0
    return {
      label: `${date.getMonth() + 1}/${date.getDate()}`,
      accessibleLabel: `${date.getMonth() + 1}月${date.getDate()}日`,
      value,
    }
  })
})
const hasSeries = computed(() => points.value.some((point) => point.value > 0))
const total = computed(() => points.value.reduce((sum, point) => sum + point.value, 0))
const average = computed(() => Math.round(total.value / rangeDays.value))
const recordedDays = computed(() => points.value.filter((point) => point.value > 0).length)
const peakPoint = computed(() => points.value.reduce<InputTrendPoint | null>(
  (peak, point) => !peak || point.value > peak.value ? point : peak,
  null,
))
const granularityLabel = computed(() => ({ minute: '分钟', hour: '小时', day: '每日', none: '无' })[props.granularity])

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}
</script>

<template>
  <section class="input-history-panel">
    <section v-if="hasSeries" class="trend-section">
      <header class="trend-heading">
        <div>
          <span class="trend-eyebrow">键盘</span>
          <h2>历史趋势</h2>
          <p>按日汇总；无记录为 0</p>
        </div>
        <div class="chart-mode" :data-active="chartMode" role="group" aria-label="图表样式">
          <button type="button" :aria-pressed="chartMode === 'line'" @click="chartMode = 'line'">
            <PhChartLine :size="16" />折线
          </button>
          <button type="button" :aria-pressed="chartMode === 'bar'" @click="chartMode = 'bar'">
            <PhChartBar :size="16" />柱状
          </button>
        </div>
      </header>

      <div class="trend-toolbar">
        <div class="range-switch" :data-active="rangeDays" role="group" aria-label="趋势时间范围">
          <button type="button" :aria-pressed="rangeDays === 7" @click="rangeDays = 7">7 天</button>
          <button type="button" :aria-pressed="rangeDays === 30" @click="rangeDays = 30">30 天</button>
        </div>
        <span class="privacy-mark"><PhShieldCheck :size="15" weight="regular" />仅聚合计数</span>
      </div>

      <InputTrendChart :points="points" :mode="chartMode" :ariaLabel="`最近 ${rangeDays} 天键盘输入趋势`" />

      <footer class="trend-summary">
        <div class="trend-total">
          <span>{{ rangeDays }} 天合计</span>
          <strong>{{ formatNumber(total) }}</strong>
          <small>次</small>
        </div>
        <dl>
          <div><dt>日均</dt><dd>{{ formatNumber(average) }}</dd></div>
          <div><dt>最高</dt><dd>{{ peakPoint?.accessibleLabel }} · {{ formatNumber(peakPoint?.value ?? 0) }}</dd></div>
          <div><dt>有记录</dt><dd>{{ recordedDays }} 天</dd></div>
          <div><dt>粒度</dt><dd>{{ granularityLabel }}</dd></div>
        </dl>
      </footer>
    </section>

    <section v-else class="history-empty">
      <PhChartLineUp :size="24" weight="regular" />
      <div><strong>暂无汇总</strong><p>切换到有记录的日期查看</p></div>
    </section>
  </section>
</template>

<style scoped>
.input-history-panel {
  min-width: 0;
  padding: 22px 24px 20px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-card);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
}

.trend-section { min-width: 0; }
.trend-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }
.trend-eyebrow { color: #2369aa; font-size: 10px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
:global(:root[data-theme="dark"]) .trend-eyebrow { color: #78baf2; }
h2 { margin: 3px 0 0; color: var(--text-primary); font-size: 20px; font-weight: 700; letter-spacing: -.45px; }
p { margin: 5px 0 0; color: var(--text-secondary); font-size: 11px; line-height: 1.55; }

/* ── Segmented controls with sliding pill indicator ─────────────────────── */
.chart-mode,
.range-switch {
  position: relative;
  display: inline-flex;
  gap: 3px;
  padding: 3px;
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  background: var(--bg-inset);
}

/* Sliding pill: absolutely-positioned pseudo-element that slides to active btn */
.chart-mode::before,
.range-switch::before {
  content: '';
  position: absolute;
  inset: 3px;
  width: calc(50% - 4.5px);   /* half-width minus padding+gap */
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  background: var(--bg-elevated);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--text-primary) 7%, transparent);
  translate: 0;
  transition: translate 220ms cubic-bezier(0.34, 1.36, 0.64, 1), box-shadow 160ms ease;
  pointer-events: none;
}

/* chart-mode: line=first btn (index 0), bar=second (index 1) */
.chart-mode[data-active="bar"]::before {
  translate: calc(100% + 3px);
}

/* range-switch: 7=first, 30=second */
.range-switch[data-active="30"]::before {
  translate: calc(100% + 3px);
}

.chart-mode button,
.range-switch button {
  position: relative;   /* sit above the pill */
  z-index: 1;
  min-height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 12px;
  border: 0;
  border-radius: 7px;
  color: var(--text-secondary);
  background: transparent;
  font: 650 11px/1 var(--font-ui);
  cursor: pointer;
  transition: color 180ms ease, transform 180ms var(--ease-out);
}

.chart-mode button:hover,
.range-switch button:hover { color: var(--text-primary); }
.chart-mode button:active,
.range-switch button:active { transform: scale(.97); }
.chart-mode button:focus-visible,
.range-switch button:focus-visible { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.chart-mode button[aria-pressed="true"],
.range-switch button[aria-pressed="true"] {
  color: var(--text-primary);
}

.trend-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 18px 0 8px;
}

.privacy-mark { display: inline-flex; align-items: center; gap: 5px; flex: 0 0 auto; color: var(--text-muted); font-size: 10px; font-weight: 600; }

.trend-summary {
  display: grid;
  grid-template-columns: minmax(190px, .72fr) minmax(0, 1.28fr);
  align-items: end;
  gap: 24px;
  margin-top: 12px;
  padding-top: 18px;
  border-top: 1px solid var(--border-soft);
}

.trend-total > span { display: block; color: var(--text-secondary); font-size: 11px; }
.trend-total strong { display: inline-block; margin-top: 4px; color: var(--text-primary); font: 700 30px/1 var(--font-data); font-variant-numeric: tabular-nums; letter-spacing: -.8px; }
.trend-total small { margin-left: 7px; color: var(--text-muted); font-size: 10px; }

dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px 18px; margin: 0; }
dl div { display: flex; justify-content: space-between; gap: 12px; padding-bottom: 7px; border-bottom: 1px solid var(--border-soft); }
dt { color: var(--text-muted); font-size: 10px; }
dd { overflow: hidden; margin: 0; color: var(--text-primary); font: 650 10px/1.2 var(--font-data); font-variant-numeric: tabular-nums; text-align: right; text-overflow: ellipsis; white-space: nowrap; }
.history-empty { min-height: 300px; display: flex; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); }
.history-empty strong { color: var(--text-primary); font-size: 12px; }

@media (max-width: 760px) {
  .input-history-panel { padding: 18px; }
  .trend-heading,
  .trend-toolbar { align-items: stretch; flex-direction: column; }
  .chart-mode,
  .range-switch { align-self: flex-start; }
  .trend-summary { grid-template-columns: 1fr; }
}

@media (prefers-reduced-motion: reduce) {
  .chart-mode button,
  .range-switch button { transition-duration: 1ms; }
  .chart-mode::before,
  .range-switch::before { transition-duration: 1ms; }
}
</style>
