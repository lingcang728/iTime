<script setup lang="ts">
import { computed } from 'vue'
import { PhBackspace, PhClock, PhGauge, PhKeyboard, PhShieldCheck } from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import { useAppStore } from '../stores/appStore'
import { hasInputData } from '../stores/dataAvailability'
import { activeInputMinutes, backspaceCorrectionRate, estimatedWordsPerMinute } from '../domain/inputFootprintModel'
import { formatNumber } from '../utils/format'

interface KeyboardKey { label: string; flex?: number }

const store = useAppStore()
const snapshot = computed(() => store.input.value)
const weekSnapshot = computed(() => store.inputWeek.value)
const inputDataAvailable = computed(() => hasInputData(store.state.inputDataStatus))
const activeMinutes = computed(() => activeInputMinutes(snapshot.value.history))
const estimatedWpm = computed(() => estimatedWordsPerMinute(snapshot.value.cumulative.keyStrokes, activeMinutes.value))
const keyCounts = computed(() => new Map(snapshot.value.singleKeys.map((item) => [item.key, item.count])))
const correctionRate = computed(() => {
  const value = backspaceCorrectionRate(snapshot.value.cumulative.keyStrokes, snapshot.value.singleKeys, snapshot.value.capabilities.keyHeatmap)
  return value === null ? null : value * 100
})
const maxKeyCount = computed(() => Math.max(1, ...snapshot.value.singleKeys.map((item) => item.count)))
const maxShortcutCount = computed(() => Math.max(1, ...snapshot.value.shortcuts.map((item) => item.count)))
const detailStartLabel = computed(() => snapshot.value.detailAvailableFrom === null ? null : new Date(snapshot.value.detailAvailableFrom).toLocaleDateString('zh-CN'))
const pageSubtitle = computed(() => ({
  ready: '看见输入节奏，不保存输入内容、按键顺序或逐键时间戳',
  preview: '浏览器预览使用演示聚合；桌面版只在本机保存日级键位统计',
  loading: '正在连接 iTime 本机键盘计数器',
  unavailable: '键盘计数器暂时不可用',
}[store.state.inputDataStatus]))

const keyboardRows: KeyboardKey[][] = [
  [{ label: '`' }, ...'1234567890'.split('').map((label) => ({ label })), { label: '-' }, { label: '=' }, { label: 'Backspace', flex: 2 }],
  [{ label: 'Tab', flex: 1.45 }, ...'QWERTYUIOP'.split('').map((label) => ({ label })), { label: '[' }, { label: ']' }, { label: '\\', flex: 1.55 }],
  [{ label: 'Caps', flex: 1.8 }, ...'ASDFGHJKL'.split('').map((label) => ({ label })), { label: ';' }, { label: "'" }, { label: 'Enter', flex: 2.05 }],
  [{ label: 'Shift', flex: 2.35 }, ...'ZXCVBNM'.split('').map((label) => ({ label })), { label: ',' }, { label: '.' }, { label: '/' }, { label: 'Shift', flex: 2.45 }],
  [{ label: 'Ctrl', flex: 1.35 }, { label: 'Win', flex: 1.2 }, { label: 'Alt', flex: 1.2 }, { label: 'Space', flex: 6.8 }, { label: 'Alt', flex: 1.2 }, { label: 'Win', flex: 1.2 }, { label: 'Ctrl', flex: 1.35 }],
]

const weekDays = computed(() => {
  const end = new Date(`${store.state.selectedDate}T12:00:00`)
  return Array.from({ length: 7 }, (_, index) => {
    const date = new Date(end)
    date.setDate(end.getDate() - (6 - index))
    const key = localDate(date)
    const hours = Array.from({ length: 24 }, (_, hour) => weekSnapshot.value.history
      .filter((point) => localDate(new Date(point.start)) === key && new Date(point.start).getHours() === hour)
      .reduce((total, point) => total + point.keyStrokes, 0))
    return { key, label: `${date.getMonth() + 1}/${date.getDate()} ${weekday(date)}`, hours }
  })
})
const maxRhythm = computed(() => Math.max(1, ...weekDays.value.flatMap((day) => day.hours)))
const minuteDensity = computed(() => {
  const bins = Array.from({ length: 96 }, () => 0)
  for (const point of snapshot.value.history) {
    const date = new Date(point.start)
    const index = date.getHours() * 4 + Math.floor(date.getMinutes() / 15)
    bins[index] += point.keyStrokes
  }
  return bins
})
const maxMinuteDensity = computed(() => Math.max(1, ...minuteDensity.value))

function localDate(value: Date): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`
}

function weekday(value: Date): string {
  return ['周日', '周一', '周二', '周三', '周四', '周五', '周六'][value.getDay()]
}

function heatLevel(value: number, maximum: number): number {
  if (!value) return 0
  return Math.max(1, Math.min(5, Math.ceil(value / maximum * 5)))
}

function keyCount(label: string): number {
  return keyCounts.value.get(label) ?? 0
}

function metricValue(value: string): string { return inputDataAvailable.value ? value : '—' }
function metricDetail(detail: string): string { return inputDataAvailable.value ? detail : store.state.inputDataMessage }
</script>

<template>
  <section class="page input-page">
    <PageHeader title="输入足迹" :subtitle="pageSubtitle" />

    <div class="metrics-grid metrics-grid--four input-metrics">
      <MetricCard label="总输入字数" :value="metricValue(formatNumber(snapshot.cumulative.keyStrokes))" :detail="metricDetail('字符与标点键的分钟总量')" :icon="PhKeyboard" tone="accent" visual="bars" />
      <MetricCard label="活跃输入分钟" :value="metricValue(`${activeMinutes} 分`)" :detail="metricDetail('至少出现一次字符键的分钟')" :icon="PhClock" visual="bars" />
      <MetricCard label="估算输入速度" :value="metricValue(`${Math.round(estimatedWpm)} WPM`)" :detail="metricDetail('按 5 个字符折算一个单词')" :icon="PhGauge" visual="bars" />
      <MetricCard label="退格修正率" :value="metricValue(correctionRate === null ? '—' : `${correctionRate.toFixed(1)}%`)" :detail="metricDetail(correctionRate === null ? '升级前无逐键明细' : '退格 ÷（字符键 + 退格）')" :icon="PhBackspace" visual="ring" />
    </div>

    <div v-if="!inputDataAvailable" class="section-state input-source-state" :data-state="store.state.inputDataStatus">
      <strong>{{ store.state.inputDataStatus === 'loading' ? '正在读取输入记录' : '输入记录暂不可用' }}</strong><span>{{ store.state.inputDataMessage }}</span>
    </div>
    <template v-else>
      <div class="input-main-grid">
        <article class="keyboard-card card">
          <div class="section-heading"><div><h2>键盘热力图</h2><p>按本地日期累计次数；不保存按键顺序</p></div><span class="privacy-chip"><PhShieldCheck :size="14" />日级聚合</span></div>
          <div v-if="snapshot.capabilities.keyHeatmap && snapshot.singleKeys.length" class="keyboard-board" role="group" aria-label="键盘按键热力图">
            <div v-for="(row, rowIndex) in keyboardRows" :key="rowIndex" class="keyboard-row">
              <button v-for="(key, keyIndex) in row" :key="`${key.label}-${keyIndex}`" type="button" :style="{ flex: key.flex ?? 1 }" :data-heat="heatLevel(keyCount(key.label), maxKeyCount)" :aria-label="`${key.label}，${keyCount(key.label)} 次`">
                <span>{{ key.label }}</span><small v-if="keyCount(key.label)">{{ formatNumber(keyCount(key.label)) }}</small>
              </button>
            </div>
            <div class="keyboard-legend"><span>低频</span><i v-for="level in [0,1,2,3,4,5]" :key="level" :data-heat="level"></i><span>高频</span><em>颜色越深，击键越频繁</em></div>
          </div>
          <div v-else class="section-state detail-empty">
            <strong>{{ snapshot.detailAvailableFrom ? '这一天没有键位明细' : '逐键热力从升级后开始采集' }}</strong>
            <span>{{ snapshot.detailAvailableFrom ? `逐键聚合从 ${detailStartLabel} 起可用；当前日期没有记录。` : '旧记录仍用于总量和输入节奏，但不会生成模拟键位分布。' }}</span>
          </div>
        </article>

        <article class="rhythm-card card">
          <div class="section-heading"><div><h2>七日输入节奏</h2><p>每格代表一小时，颜色只表示相对输入密度</p></div></div>
          <div class="rhythm-axis" aria-hidden="true"><span v-for="label in ['00','04','08','12','16','20','24']" :key="label">{{ label }}:00</span></div>
          <div class="rhythm-grid" role="img" aria-label="过去七日每小时输入密度">
            <div v-for="day in weekDays" :key="day.key" class="rhythm-row"><span>{{ day.label }}</span><div><i v-for="(value, hour) in day.hours" :key="hour" :data-heat="heatLevel(value, maxRhythm)" :title="`${day.label} ${String(hour).padStart(2, '0')}:00，${value} 次`"></i></div></div>
          </div>
          <div class="density-heading"><strong>当日分钟密度</strong><span>每 15 分钟</span></div>
          <div class="minute-density" role="img" aria-label="当日每十五分钟输入密度"><i v-for="(value, index) in minuteDensity" :key="index" :data-heat="heatLevel(value, maxMinuteDensity)" :title="`${String(Math.floor(index / 4)).padStart(2, '0')}:${String(index % 4 * 15).padStart(2, '0')}，${value} 次`"></i></div>
        </article>
      </div>

      <article class="shortcut-card card">
        <div class="section-heading"><div><h2>快捷键排行</h2><p>只统计含 Ctrl、Alt 或 Win 的规范化组合</p></div></div>
        <div v-if="snapshot.capabilities.functionalShortcuts && snapshot.shortcuts.length" class="shortcut-list">
          <div v-for="(item, index) in snapshot.shortcuts.slice(0, 8)" :key="item.shortcut"><span>{{ index + 1 }}</span><kbd>{{ item.shortcut }}</kbd><i><b :style="{ width: `${item.count / maxShortcutCount * 100}%` }"></b></i><strong>{{ formatNumber(item.count) }} 次</strong></div>
        </div>
        <div v-else class="section-state shortcut-empty"><strong>{{ snapshot.detailAvailableFrom ? '这一天没有快捷键记录' : '快捷键排行从升级后开始采集' }}</strong><span>不会根据总击键数推算或生成组合键。</span></div>
      </article>

      <footer class="input-privacy"><PhShieldCheck :size="16" /><span>键位和快捷键仅按本地日期累计；不保存按键顺序、逐键时间戳、原始虚拟键码、输入文本、IME 结果或剪贴板内容。</span></footer>
    </template>
  </section>
</template>

<style scoped src="./input-footprint-page.css"></style>
