<script setup lang="ts">
import { computed } from 'vue'
import type { InputActivitySnapshot, InputKeyCount } from '../../providers/inputActivity'
import { uiIcons } from '../../data/uiIcons'
import { formatNumber } from '../../utils/format'

const props = defineProps<{
  snapshot: InputActivitySnapshot
  heatmapEnabled: boolean
  shortcutsEnabled: boolean
}>()

const maximum = computed(() => Math.max(1, ...props.snapshot.singleKeys.map((key) => key.count)))
const heatKeys = computed(() => props.snapshot.singleKeys.slice(0, 12))
const rankedKeys = computed(() => props.snapshot.singleKeys.slice(0, 10))
const visibleShortcuts = computed(() => props.snapshot.shortcuts.slice(0, 8))
const canShowKeys = computed(() => props.heatmapEnabled && props.snapshot.capabilities.keyHeatmap && heatKeys.value.length > 0)
const canShowShortcuts = computed(() => props.shortcutsEnabled && props.snapshot.capabilities.functionalShortcuts && visibleShortcuts.value.length > 0)
const unavailableMessage = computed(() => props.snapshot.capabilities.historyGranularity === 'day'
  ? 'KeyStats 仅保留当天的键位明细；历史日期仍可查看总敲击数。'
  : '当前数据源没有提供单键明细。')

function heatStyle(key: InputKeyCount): Record<string, string> {
  const ratio = key.count / maximum.value
  return {
    '--heat-fill': `color-mix(in srgb, var(--accent-green) ${Math.round(12 + ratio * 28)}%, var(--bg-soft))`,
    gridColumn: key.key.toLowerCase() === 'space' ? 'span 2' : 'span 1',
  }
}

function shortcutParts(shortcut: string): string[] {
  return shortcut.split('+').map((part) => part.trim()).filter(Boolean)
}
</script>

<template>
  <section class="keyboard-layout">
    <article class="card heatmap-card">
      <header>
        <div class="card-title-with-icon">
          <img class="card-title-icon" :src="uiIcons.inputHeatmap" alt="" draggable="false" />
          <div><h2>键盘热力图</h2><p>按单键累计次数着色，不保存输入内容</p></div>
        </div>
        <span>{{ canShowKeys ? '今日键位' : '无明细' }}</span>
      </header>
      <div v-if="canShowKeys" class="key-grid" role="list" aria-label="今日键位热力图">
        <div
          v-for="key in heatKeys"
          :key="key.key"
          class="key-cap"
          role="listitem"
          tabindex="0"
          :style="heatStyle(key)"
          :aria-label="`${key.key}，${formatNumber(key.count)} 次`"
        >
          <kbd>{{ key.key }}</kbd><strong>{{ formatNumber(key.count) }}</strong>
        </div>
      </div>
      <div v-else class="detail-empty">
        <img class="detail-empty__art" :src="uiIcons.inputHeatmap" alt="" draggable="false" />
        <div><strong>{{ heatmapEnabled ? '此日期没有键位明细' : '键盘热力图已关闭' }}</strong><p>{{ heatmapEnabled ? unavailableMessage : '可在设置中重新开启本地键位聚合。' }}</p></div>
      </div>
    </article>

    <article class="card ranking-card">
      <header>
        <div class="card-title-with-icon">
          <img class="card-title-icon" :src="uiIcons.inputTopKeys" alt="" draggable="false" />
          <div><h2>Top 键位</h2><p>最多展示 10 个今日键位</p></div>
        </div>
      </header>
      <ol v-if="canShowKeys">
        <li v-for="(key, index) in rankedKeys" :key="key.key">
          <span>{{ String(index + 1).padStart(2, '0') }}</span>
          <kbd>{{ key.key }}</kbd>
          <i aria-hidden="true"><em :style="{ width: `${key.count / maximum * 100}%` }" /></i>
          <strong>{{ formatNumber(key.count) }}</strong>
        </li>
      </ol>
      <div v-else class="compact-empty">历史日期不提供 Top 键位</div>
    </article>

    <article class="card shortcut-card">
      <header>
        <div class="card-title-with-icon">
          <img class="card-title-icon" :src="uiIcons.inputShortcuts" alt="" draggable="false" />
          <div><h2>功能组合键</h2><p>明确组合键的本地累计</p></div>
        </div>
      </header>
      <div v-if="canShowShortcuts" class="shortcut-list">
        <div v-for="shortcut in visibleShortcuts" :key="shortcut.shortcut">
          <span class="shortcut-keys">
            <template v-for="(part, index) in shortcutParts(shortcut.shortcut)" :key="`${shortcut.shortcut}-${part}`">
              <i v-if="index" aria-hidden="true">+</i><kbd>{{ part }}</kbd>
            </template>
          </span>
          <strong>{{ formatNumber(shortcut.count) }}</strong>
        </div>
      </div>
      <div v-else class="detail-empty detail-empty--small">
        <img class="detail-empty__art" :src="uiIcons.inputShortcuts" alt="" draggable="false" />
        <div><strong>{{ shortcutsEnabled ? '此日期没有组合键明细' : '组合键统计已关闭' }}</strong><p>{{ shortcutsEnabled ? unavailableMessage : '可在设置中重新开启。' }}</p></div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.keyboard-layout { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(240px, .72fr) minmax(230px, .68fr); gap: 12px; margin-top: 12px; }
.keyboard-layout > article { min-width: 0; padding: 19px; }
header { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 16px; }
h2 { margin: 0; font-size: 15px; letter-spacing: -.2px; }
p { margin: 5px 0 0; color: var(--text-secondary); font-size: 9px; line-height: 1.5; }
header > span { flex: 0 0 auto; padding: 5px 8px; border-radius: 999px; color: var(--accent-green-strong); background: var(--accent-green-soft); font-size: 9px; }
.key-grid { display: grid; grid-template-columns: repeat(6, minmax(52px, 1fr)); gap: 7px; }
.key-cap { --heat-fill: var(--bg-soft); min-height: 58px; display: grid; align-content: space-between; padding: 10px 11px 8px; border: 1px solid color-mix(in srgb, var(--accent-green) 22%, var(--border-strong)); border-bottom-width: 3px; border-radius: 9px; background: var(--heat-fill); box-shadow: inset 0 1px rgba(255, 255, 255, .45); transition: transform 150ms var(--ease-out), border-color 150ms ease; }
.key-cap:hover,
.key-cap:focus-visible { transform: translateY(-2px); border-color: color-mix(in srgb, var(--accent-green) 48%, var(--border-strong)); }
kbd { overflow: hidden; color: var(--text-primary); font: 650 10px/1.2 var(--font-ui); text-overflow: ellipsis; white-space: nowrap; }
.key-cap strong { color: var(--text-secondary); font: 650 9px/1 var(--font-data); }
.ranking-card ol { display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; }
.ranking-card li { display: grid; grid-template-columns: 20px 62px minmax(30px, 1fr) 50px; align-items: center; gap: 8px; font-size: 9px; }
.ranking-card li > span { color: var(--text-muted); font-variant-numeric: tabular-nums; }
.ranking-card li > kbd { padding: 6px 7px; border: 1px solid var(--border-soft); border-bottom-width: 2px; border-radius: 7px; background: var(--bg-soft); text-align: center; }
.ranking-card li > i { height: 5px; overflow: hidden; border-radius: 999px; background: var(--bg-soft); }
.ranking-card li > i em { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent-blue), color-mix(in srgb, var(--accent-blue) 58%, var(--accent-cyan))); }
.ranking-card li > strong { text-align: right; font-variant-numeric: tabular-nums; }
.shortcut-list { display: grid; gap: 9px; }
.shortcut-list > div { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding-bottom: 9px; border-bottom: 1px solid var(--border-soft); }
.shortcut-list > div:last-child { border-bottom: 0; }
.shortcut-keys { min-width: 0; display: flex; align-items: center; gap: 4px; }
.shortcut-keys kbd { min-width: 28px; padding: 6px 7px; border: 1px solid var(--border-strong); border-bottom-width: 2px; border-radius: 6px; background: var(--bg-soft); text-align: center; }
.shortcut-keys i { color: var(--text-muted); font-size: 9px; font-style: normal; }
.shortcut-list strong { font: 700 10px/1 var(--font-data); font-variant-numeric: tabular-nums; }
.card-title-with-icon { display: flex; align-items: flex-start; gap: 9px; min-width: 0; }
.card-title-icon { width: 26px; height: 26px; flex: 0 0 26px; margin-top: 1px; object-fit: contain; filter: drop-shadow(0 1px 2px rgba(34, 38, 45, 0.1)); }
.detail-empty__art { width: 36px; height: 36px; flex: 0 0 36px; object-fit: contain; filter: drop-shadow(0 2px 3px rgba(34, 38, 45, 0.1)); opacity: 0.92; }
.detail-empty { min-height: 146px; display: flex; align-items: center; justify-content: center; gap: 11px; padding: 14px; border: 1px dashed var(--border-strong); border-radius: 11px; color: var(--text-muted); background: var(--bg-soft); }
.detail-empty strong { display: block; color: var(--text-primary); font-size: 11px; }
.detail-empty p { max-width: 250px; }
.detail-empty--small { min-height: 120px; }
.compact-empty { display: grid; place-items: center; min-height: 148px; border-radius: 10px; color: var(--text-muted); background: var(--bg-soft); font-size: 10px; }
@media (max-width: 1080px) { .keyboard-layout { grid-template-columns: 1fr 1fr; } .heatmap-card { grid-column: 1 / -1; } }
@media (max-width: 760px) { .keyboard-layout { grid-template-columns: 1fr; } .heatmap-card { grid-column: auto; } .key-grid { grid-template-columns: repeat(4, minmax(52px, 1fr)); } }
</style>
