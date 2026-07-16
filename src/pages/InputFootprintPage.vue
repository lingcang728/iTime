<script setup lang="ts">
import { computed } from 'vue'
import { PhArrowsDownUp, PhCoffee, PhCursorClick, PhKeyboard, PhMouse, PhMouseLeftClick, PhMouseRightClick, PhShieldCheck } from '@phosphor-icons/vue'
import MetricCard from '../components/MetricCard.vue'
import PageHeader from '../components/PageHeader.vue'
import SparkLine from '../components/SparkLine.vue'
import { useAppStore } from '../stores/appStore'
import { formatDistance, formatNumber } from '../utils/format'

const store = useAppStore()
const hourlyKeys = computed(() => store.input.value.history.map((point) => point.keyStrokes))
const maxHourly = computed(() => Math.max(1, ...hourlyKeys.value))
const rhythm = computed(() => store.input.value.history.map((point) => ({
  hour: new Date(point.start).getHours(),
  value: point.keyStrokes,
  height: Math.max(4, point.keyStrokes / maxHourly.value * 100),
})))
const maxKey = computed(() => Math.max(1, ...store.input.value.singleKeys.map((key) => key.count)))
const visibleKeys = computed(() => store.state.heatmapEnabled ? store.input.value.singleKeys : [])
const clickValue = (value: number | null) => value === null ? '—' : formatNumber(value)
</script>

<template>
  <section class="page input-page">
    <PageHeader title="输入足迹" subtitle="补充理解你的输入节奏；只保存聚合统计，不记录文字内容" />
    <div class="metrics-grid input-metrics">
      <MetricCard label="键盘敲击" :value="formatNumber(store.input.value.cumulative.keyStrokes)" :detail="store.input.value.source" :icon="PhKeyboard" tone="green" />
      <MetricCard label="鼠标左键点击" :value="clickValue(store.input.value.cumulative.leftClicks)" :detail="store.input.value.cumulative.leftClicks === null ? '该日期仅有合并点击' : '累计快照'" :icon="PhMouseLeftClick" tone="blue" />
      <MetricCard label="鼠标右键点击" :value="clickValue(store.input.value.cumulative.rightClicks)" :detail="store.input.value.cumulative.rightClicks === null ? '该日期仅有合并点击' : '累计快照'" :icon="PhMouseRightClick" tone="violet" />
      <MetricCard label="鼠标移动" :value="formatDistance(store.input.value.cumulative.mouseDistance)" detail="聚合距离" :icon="PhMouse" tone="cyan" />
      <MetricCard label="滚动距离" :value="formatNumber(store.input.value.cumulative.scrollDistance)" detail="聚合滚动单位" :icon="PhArrowsDownUp" tone="orange" />
    </div>
    <div class="input-main-grid">
      <article class="card input-trend-card">
        <div class="section-heading"><div><h2>今日输入趋势</h2><p>按小时聚合，不保存原始键盘事件</p></div><span class="local-badge"><PhShieldCheck :size="15" />仅本地</span></div>
        <div class="input-spark"><SparkLine :values="hourlyKeys" color="#54ae7e" /></div>
        <div class="input-trend-foot"><span>08:00</span><span>12:00</span><span>16:00</span><span>20:00</span></div>
      </article>
      <article class="card rhythm-card">
        <div class="section-heading"><div><h2>今日输入节奏</h2><p>活跃时段与自然停顿</p></div></div>
        <div class="rhythm-bars"><span v-for="point in rhythm" :key="point.hour" :style="{ height: `${point.height}%` }" :title="`${point.hour}:00 · ${point.value} 次敲击`"></span></div>
        <div class="rhythm-labels"><span v-for="point in rhythm" :key="point.hour">{{ point.hour }}</span></div>
      </article>
      <article class="card factual-insight">
        <PhCoffee :size="27" weight="duotone" />
        <span>可观察事实</span><h2>最长连续输入 47 分钟</h2><p>10:00—11:00 输入最集中；午间出现 36 分钟无输入间隔。此处不作健康或风险判断。</p>
      </article>
    </div>
    <div class="input-bottom-grid">
      <article class="card keyboard-card">
        <div class="section-heading"><div><h2>键盘热力图</h2><p>仅保存单键累计次数</p></div><span class="capability-state">{{ store.state.heatmapEnabled ? '已开启' : '已关闭' }}</span></div>
        <div v-if="visibleKeys.length" class="keyboard-heatmap">
          <span v-for="key in visibleKeys" :key="key.key" :style="{ '--heat': `${0.15 + key.count / maxKey * 0.85}` }"><strong>{{ key.key }}</strong><small>{{ formatNumber(key.count) }}</small></span>
        </div>
        <div v-else class="privacy-empty"><PhKeyboard :size="27" /><span>键盘热力图统计已关闭</span></div>
      </article>
      <article class="card key-ranking-card">
        <div class="section-heading"><div><h2>Top 键位</h2><p>累计分布</p></div></div>
        <ol><li v-for="(key, index) in store.input.value.singleKeys.slice(0, 6)" :key="key.key"><span>{{ index + 1 }}</span><kbd>{{ key.key }}</kbd><i><em :style="{ width: `${key.count / maxKey * 100}%` }"></em></i><strong>{{ formatNumber(key.count) }}</strong></li></ol>
      </article>
      <article class="card shortcut-card">
        <div class="section-heading"><div><h2>功能组合键</h2><p>只统计明确功能组合</p></div></div>
        <div v-if="store.state.shortcutsEnabled" class="shortcut-list"><div v-for="shortcut in store.input.value.shortcuts" :key="shortcut.shortcut"><kbd>{{ shortcut.shortcut }}</kbd><strong>{{ shortcut.count }}</strong></div></div>
        <div v-else class="privacy-empty"><PhCursorClick :size="27" /><span>组合键统计已关闭</span></div>
      </article>
    </div>
  </section>
</template>
