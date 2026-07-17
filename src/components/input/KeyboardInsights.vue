<script setup lang="ts">
import { computed } from 'vue'
import type { InputActivitySnapshot } from '../../providers/inputActivity'
import { uiIcons } from '../../data/uiIcons'
import { formatNumber } from '../../utils/format'

const props = defineProps<{
  snapshot: InputActivitySnapshot
  heatmapEnabled: boolean
  shortcutsEnabled: boolean
}>()

interface KeyboardKey {
  label: string
  aliases?: string[]
  units?: number
}

interface KeyboardSection {
  id: 'function' | 'main' | 'navigation' | 'numpad'
  label: string
  rows: KeyboardKey[][]
}

const keyboardSections: KeyboardSection[] = [
  {
    id: 'function', label: '功能区', rows: [[
      { label: 'Esc', aliases: ['Escape'], units: 1.2 },
      ...Array.from({ length: 12 }, (_, index) => ({ label: `F${index + 1}` })),
      { label: 'PrtSc', aliases: ['PrintScreen'] }, { label: 'ScrLk', aliases: ['ScrollLock'] }, { label: 'Pause' },
    ]],
  },
  {
    id: 'main', label: '主键区', rows: [
      [...['`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '='].map((label) => ({ label })), { label: 'Backspace', aliases: ['Backsp...', 'Bksp'], units: 2.1 }],
      [{ label: 'Tab', units: 1.5 }, ...'QWERTYUIOP'.split('').map((label) => ({ label })), { label: '[' }, { label: ']' }, { label: '\\', units: 1.5 }],
      [{ label: 'Caps', aliases: ['CapsLock'], units: 1.8 }, ...'ASDFGHJKL'.split('').map((label) => ({ label })), { label: ';' }, { label: "'" }, { label: 'Enter', units: 2.2 }],
      [{ label: 'Shift', units: 2.3 }, ...'ZXCVBNM'.split('').map((label) => ({ label })), { label: ',' }, { label: '.' }, { label: '/' }, { label: 'Shift', aliases: ['RShift'], units: 2.7 }],
      [{ label: 'Ctrl', units: 1.3 }, { label: 'Win', units: 1.2 }, { label: 'Alt', units: 1.2 }, { label: 'Space', aliases: ['Spacebar'], units: 6.2 }, { label: 'Alt', aliases: ['RAlt'], units: 1.2 }, { label: 'Fn', units: 1.1 }, { label: 'Menu', units: 1.1 }, { label: 'Ctrl', aliases: ['RCtrl'], units: 1.3 }],
    ],
  },
  {
    id: 'navigation', label: '导航区', rows: [
      [{ label: 'Ins', aliases: ['Insert'] }, { label: 'Home' }, { label: 'PgUp', aliases: ['PageUp'] }],
      [{ label: 'Del', aliases: ['Delete'] }, { label: 'End' }, { label: 'PgDn', aliases: ['PageDown'] }],
      [{ label: '↑', aliases: ['ArrowUp', 'Up'] }],
      [{ label: '←', aliases: ['ArrowLeft', 'Left'] }, { label: '↓', aliases: ['ArrowDown', 'Down'] }, { label: '→', aliases: ['ArrowRight', 'Right'] }],
    ],
  },
  {
    id: 'numpad', label: '数字区', rows: [
      [{ label: 'Num' }, { label: '/' }, { label: '*' }, { label: '-' }],
      ['7', '8', '9', '+'].map((label) => ({ label })),
      ['4', '5', '6', '+'].map((label) => ({ label })),
      ['1', '2', '3', 'Enter'].map((label) => ({ label })),
      [{ label: '0', units: 2 }, { label: '.' }, { label: 'Enter' }],
    ],
  },
]

const maximum = computed(() => props.snapshot.singleKeys.reduce((current, key) => Math.max(current, key.count), 1))
const rankedKeys = computed(() => props.snapshot.singleKeys.slice(0, 10))
const visibleShortcuts = computed(() => props.snapshot.shortcuts.slice(0, 8))
const keyCounts = computed(() => new Map(props.snapshot.singleKeys.map((key) => [normalizeKey(key.key), key.count])))
const canShowKeys = computed(() => props.heatmapEnabled && props.snapshot.capabilities.keyHeatmap && props.snapshot.singleKeys.length > 0)
const canShowShortcuts = computed(() => props.shortcutsEnabled && props.snapshot.capabilities.functionalShortcuts && visibleShortcuts.value.length > 0)
const unavailableMessage = computed(() => props.snapshot.capabilities.historyGranularity === 'day'
  ? 'iTime 仅保留当天的键位明细；历史日期仍可查看总敲击数。'
  : '当前数据源没有提供单键明细。')

function normalizeKey(value: string): string {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, '')
}

function keyCount(key: KeyboardKey): number {
  for (const candidate of [key.label, ...(key.aliases ?? [])]) {
    const count = keyCounts.value.get(normalizeKey(candidate))
    if (count !== undefined) return count
  }
  return 0
}

function heatStyle(key: KeyboardKey): Record<string, string> {
  const count = keyCount(key)
  const ratio = count / maximum.value
  return {
    '--heat-fill': `color-mix(in srgb, var(--accent-green) ${Math.round(4 + ratio * 52)}%, var(--bg-soft))`,
    '--key-units': String(key.units ?? 1),
  }
}

function shortcutParts(shortcut: string): string[] {
  return shortcut.split('+').map((part) => part.trim()).filter(Boolean)
}
</script>

<template>
  <section class="keyboard-insights">
    <article class="card heatmap-card">
      <header>
        <div class="card-title-with-icon">
          <img class="card-title-icon" :src="uiIcons.inputHeatmap" alt="" draggable="false" />
          <div><h2>键盘热力图</h2><p>按真实 Windows 键盘分区排列；颜色越亮，使用越频繁</p></div>
        </div>
        <span>{{ canShowKeys ? '今日键位' : '无明细' }}</span>
      </header>
      <div v-if="canShowKeys" class="keyboard-board" role="group" aria-label="今日真实键盘布局热力图">
        <section v-for="section in keyboardSections" :key="section.id" class="keyboard-section" :class="`keyboard-section--${section.id}`">
          <span>{{ section.label }}</span>
          <div class="keyboard-rows">
            <div v-for="(row, rowIndex) in section.rows" :key="rowIndex" class="keyboard-row">
              <button
                v-for="(key, keyIndex) in row"
                :key="`${rowIndex}-${key.label}-${keyIndex}`"
                type="button"
                class="key-cap"
                :class="{ 'has-activity': keyCount(key) > 0 }"
                :style="heatStyle(key)"
                :aria-label="`${key.label}，${formatNumber(keyCount(key))} 次`"
              ><kbd>{{ key.label }}</kbd><small v-if="keyCount(key)">{{ formatNumber(keyCount(key)) }}</small></button>
            </div>
          </div>
        </section>
        <div class="keyboard-scale" aria-hidden="true"><span>较少</span><i></i><span>较多</span></div>
      </div>
      <div v-else class="detail-empty">
        <img class="detail-empty__art" :src="uiIcons.inputHeatmap" alt="" draggable="false" />
        <div><strong>{{ heatmapEnabled ? '此日期没有键位明细' : '键盘热力图已关闭' }}</strong><p>{{ heatmapEnabled ? unavailableMessage : '可在设置中重新开启本地键位聚合。' }}</p></div>
      </div>
    </article>

    <div class="keyboard-support-grid">
      <article class="card ranking-card">
        <header>
          <div class="card-title-with-icon"><img class="card-title-icon" :src="uiIcons.inputTopKeys" alt="" draggable="false" /><div><h2>Top 键位</h2><p>横向比较今天使用最多的 10 个键位</p></div></div>
        </header>
        <ol v-if="canShowKeys">
          <li v-for="(key, index) in rankedKeys" :key="key.key">
            <span>{{ String(index + 1).padStart(2, '0') }}</span><kbd>{{ key.key }}</kbd><strong>{{ formatNumber(key.count) }}</strong>
            <i aria-hidden="true"><em :style="{ width: `${key.count / maximum * 100}%` }" /></i>
          </li>
        </ol>
        <div v-else class="compact-empty">历史日期不提供 Top 键位</div>
      </article>

      <article class="card shortcut-card">
        <header>
          <div class="card-title-with-icon"><img class="card-title-icon" :src="uiIcons.inputShortcuts" alt="" draggable="false" /><div><h2>功能组合键</h2><p>常用功能组合横向排列，便于快速比较</p></div></div>
        </header>
        <div v-if="canShowShortcuts" class="shortcut-list">
          <div v-for="shortcut in visibleShortcuts" :key="shortcut.shortcut">
            <span class="shortcut-keys"><template v-for="(part, index) in shortcutParts(shortcut.shortcut)" :key="`${shortcut.shortcut}-${part}`"><i v-if="index" aria-hidden="true">+</i><kbd>{{ part }}</kbd></template></span>
            <strong>{{ formatNumber(shortcut.count) }}</strong>
          </div>
        </div>
        <div v-else class="detail-empty detail-empty--small">
          <img class="detail-empty__art" :src="uiIcons.inputShortcuts" alt="" draggable="false" />
          <div><strong>{{ shortcutsEnabled ? '此日期没有组合键明细' : '组合键统计已关闭' }}</strong><p>{{ shortcutsEnabled ? unavailableMessage : '可在设置中重新开启。' }}</p></div>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.keyboard-layout { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(240px, .72fr) minmax(230px, .68fr); gap: 12px; margin-top: 12px; }
.keyboard-layout > article { min-width: 0; padding: 19px; }
header { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 16px; }
h2 { margin: 0; font-size: 15px; letter-spacing: -.2px; }
p { margin: 5px 0 0; color: var(--text-secondary); font-size: 10px; line-height: 1.5; }
header > span { flex: 0 0 auto; padding: 5px 8px; border-radius: 999px; color: var(--accent-green-strong); background: var(--accent-green-soft); font-size: 10px; }
.key-grid { display: grid; grid-template-columns: repeat(6, minmax(52px, 1fr)); gap: 7px; }
.key-cap { --heat-fill: var(--bg-soft); min-height: 58px; display: grid; align-content: space-between; padding: 10px 11px 8px; border: 1px solid color-mix(in srgb, var(--accent-green) 22%, var(--border-strong)); border-bottom-width: 3px; border-radius: 9px; background: var(--heat-fill); box-shadow: inset 0 1px rgba(255, 255, 255, .45); transition: transform 150ms var(--ease-out), border-color 150ms ease; }
.key-cap:hover,
.key-cap:focus-visible { transform: translateY(-2px); border-color: color-mix(in srgb, var(--accent-green) 48%, var(--border-strong)); }
kbd { overflow: hidden; color: var(--text-primary); font: 650 10px/1.2 var(--font-ui); text-overflow: ellipsis; white-space: nowrap; }
.key-cap strong { color: var(--text-primary); font: 650 10px/1 var(--font-data); }
.ranking-card ol { display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; }
.ranking-card li { display: grid; grid-template-columns: 20px 62px minmax(30px, 1fr) 50px; align-items: center; gap: 8px; font-size: 10px; }
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
.shortcut-keys i { color: var(--text-muted); font-size: 10px; font-style: normal; }
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

.keyboard-insights { display: grid; gap: 12px; margin-top: 12px; }
.keyboard-insights > .card,
.keyboard-support-grid > .card { min-width: 0; padding: 17px 18px; }
.keyboard-board { display: grid; grid-template-columns: minmax(0, 4.5fr) minmax(150px, 1fr) minmax(180px, 1.25fr); gap: 12px; padding: 12px; border: 1px solid var(--border-soft); border-radius: 12px; background: color-mix(in srgb, var(--bg-inset) 72%, var(--bg-card)); }
.keyboard-section { min-width: 0; display: grid; grid-template-rows: 13px minmax(0, 1fr); gap: 6px; }
.keyboard-section > span { color: var(--text-muted); font-size: 8px; font-weight: 650; letter-spacing: .08em; }
.keyboard-section--function { grid-column: 1 / -1; }
.keyboard-rows { display: grid; gap: 5px; }
.keyboard-row { min-width: 0; display: flex; gap: 5px; }
.keyboard-section--function .keyboard-row { gap: 7px; }
.keyboard-section--navigation .keyboard-row:nth-child(3) { justify-content: center; padding-inline: calc(33.333% + 2px); }
.keyboard-section--navigation .keyboard-row:nth-child(4) { justify-content: center; }
.keyboard-board .key-cap { --heat-fill: var(--bg-soft); min-width: 0; min-height: 31px; flex: var(--key-units) 1 0; display: grid; place-items: center; align-content: center; gap: 2px; padding: 4px 3px; border: 1px solid color-mix(in srgb, var(--accent-green) 14%, var(--border-strong)); border-bottom-width: 2px; border-radius: 6px; color: var(--text-secondary); background: var(--heat-fill); box-shadow: inset 0 1px rgba(255, 255, 255, .035); cursor: default; transition: transform 150ms var(--ease-out), border-color 150ms ease, filter 150ms ease; }
.keyboard-section--function .key-cap { min-height: 27px; }
.keyboard-board .key-cap.has-activity { color: var(--text-primary); border-color: color-mix(in srgb, var(--accent-green) 42%, var(--border-strong)); cursor: help; }
.keyboard-board .key-cap:hover,
.keyboard-board .key-cap:focus-visible { transform: translateY(-2px); filter: brightness(1.06); outline: none; }
.keyboard-board .key-cap kbd { max-width: 100%; font-size: 8px; font-weight: 650; }
.keyboard-board .key-cap small { color: color-mix(in srgb, var(--text-primary) 78%, transparent); font: 700 7px/1 var(--font-data); }
.keyboard-scale { grid-column: 1 / -1; display: flex; align-items: center; gap: 8px; color: var(--text-muted); font-size: 8px; }
.keyboard-scale i { width: 150px; height: 5px; border-radius: 99px; background: linear-gradient(90deg, var(--bg-soft), color-mix(in srgb, var(--accent-green) 72%, var(--bg-soft))); }
.keyboard-support-grid { display: grid; grid-template-columns: minmax(0, 1.18fr) minmax(0, .82fr); gap: 12px; }
.keyboard-support-grid header { margin-bottom: 13px; }
.keyboard-support-grid .ranking-card ol { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 7px; }
.keyboard-support-grid .ranking-card li { min-width: 0; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 6px; padding: 8px 9px; border: 1px solid var(--border-soft); border-radius: 9px; background: var(--bg-subtle); }
.keyboard-support-grid .ranking-card li > span { align-self: center; font-size: 8px; }
.keyboard-support-grid .ranking-card li > kbd { min-width: 0; overflow: hidden; padding: 0; border: 0; background: transparent; text-align: left; text-overflow: ellipsis; }
.keyboard-support-grid .ranking-card li > strong { font-size: 9px; }
.keyboard-support-grid .ranking-card li > i { height: 3px; grid-column: 1 / -1; }
.keyboard-support-grid .shortcut-list { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 7px; }
.keyboard-support-grid .shortcut-list > div { min-width: 0; display: grid; gap: 8px; padding: 8px 9px; border: 1px solid var(--border-soft); border-radius: 9px; background: var(--bg-subtle); }
.keyboard-support-grid .shortcut-keys { flex-wrap: wrap; }
.keyboard-support-grid .shortcut-list strong { justify-self: end; color: var(--text-secondary); }
.keyboard-support-grid .detail-empty--small,
.keyboard-support-grid .compact-empty { min-height: 84px; }
@media (max-width: 1120px) {
  .keyboard-board { grid-template-columns: minmax(0, 3fr) minmax(140px, .8fr); }
  .keyboard-section--numpad { display: none; }
  .keyboard-support-grid .ranking-card ol { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .keyboard-support-grid .shortcut-list { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 760px) {
  .keyboard-board,
  .keyboard-support-grid { grid-template-columns: 1fr; }
  .keyboard-section--function { display: none; }
  .keyboard-section--navigation { grid-column: auto; }
}
</style>
