<script setup lang="ts">
import { computed } from 'vue'
import { PhCommand, PhKeyboard, PhKey } from '@phosphor-icons/vue'
import type { InputActivitySnapshot } from '../../providers/inputActivity'
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
    '--heat-fill': `color-mix(in srgb, var(--accent-green) ${Math.round(4 + ratio * 34)}%, var(--bg-soft))`,
    '--key-units': String(key.units ?? 1),
  }
}

function shortcutParts(shortcut: string): string[] {
  return shortcut.split('+').map((part) => part.trim()).filter(Boolean)
}
</script>

<template>
  <section class="keyboard-insights">
    <section class="keyboard-feature-section">
      <header class="keyboard-section-heading">
        <div class="keyboard-heading-copy">
          <PhKeyboard class="keyboard-heading-icon" :size="19" weight="regular" />
          <div><h2>键盘热力图</h2><p>按真实 Windows 键盘分区排列；颜色越亮，使用越频繁</p></div>
        </div>
        <span class="keyboard-status">{{ canShowKeys ? '今日键位' : '无明细' }}</span>
      </header>
      <div v-if="canShowKeys" class="keyboard-board" role="group" aria-label="今日真实键盘布局热力图">
        <section v-for="section in keyboardSections" :key="section.id" class="keyboard-section" :class="`keyboard-section--${section.id}`">
          <span>{{ section.label }}</span>
          <div class="keyboard-rows">
            <div v-for="(row, rowIndex) in section.rows" :key="rowIndex" class="keyboard-row">
              <span
                v-for="(key, keyIndex) in row"
                :key="`${rowIndex}-${key.label}-${keyIndex}`"
                class="key-cap"
                :class="{ 'has-activity': keyCount(key) > 0 }"
                :style="heatStyle(key)"
                :aria-label="`${key.label}，${formatNumber(keyCount(key))} 次`"
                role="img"
              ><kbd>{{ key.label }}</kbd><small v-if="keyCount(key)">{{ formatNumber(keyCount(key)) }}</small></span>
            </div>
          </div>
        </section>
        <div class="keyboard-scale" aria-hidden="true"><span>较少</span><i></i><span>较多</span></div>
      </div>
      <div v-else class="detail-empty">
        <span class="detail-empty__icon"><PhKeyboard :size="22" weight="regular" /></span>
        <div><strong>{{ heatmapEnabled ? '此日期没有键位明细' : '键盘热力图已关闭' }}</strong><p>{{ heatmapEnabled ? unavailableMessage : '可在设置中重新开启本地键位聚合。' }}</p></div>
      </div>
    </section>

    <div class="keyboard-support-grid">
      <section class="ranking-section">
        <header class="keyboard-section-heading">
          <div class="keyboard-heading-copy"><PhKey class="keyboard-heading-icon" :size="19" weight="regular" /><div><h2>Top 键位</h2><p>比较今天使用最多的 10 个键位</p></div></div>
        </header>
        <ol v-if="canShowKeys">
          <li v-for="(key, index) in rankedKeys" :key="key.key">
            <span>{{ String(index + 1).padStart(2, '0') }}</span><kbd>{{ key.key }}</kbd><strong>{{ formatNumber(key.count) }}</strong>
            <i aria-hidden="true"><em :style="{ width: `${key.count / maximum * 100}%` }" /></i>
          </li>
        </ol>
        <div v-else class="compact-empty">历史日期不提供 Top 键位</div>
      </section>

      <section class="shortcut-section">
        <header class="keyboard-section-heading">
          <div class="keyboard-heading-copy"><PhCommand class="keyboard-heading-icon" :size="19" weight="regular" /><div><h2>功能组合键</h2><p>常用功能组合按使用次数归纳</p></div></div>
        </header>
        <div v-if="canShowShortcuts" class="shortcut-list">
          <div v-for="shortcut in visibleShortcuts" :key="shortcut.shortcut">
            <span class="shortcut-keys"><template v-for="(part, index) in shortcutParts(shortcut.shortcut)" :key="`${shortcut.shortcut}-${part}`"><i v-if="index" aria-hidden="true">+</i><kbd>{{ part }}</kbd></template></span>
            <strong>{{ formatNumber(shortcut.count) }}</strong>
          </div>
        </div>
        <div v-else class="detail-empty detail-empty--small">
          <span class="detail-empty__icon"><PhCommand :size="20" weight="regular" /></span>
          <div><strong>{{ shortcutsEnabled ? '此日期没有组合键明细' : '组合键统计已关闭' }}</strong><p>{{ shortcutsEnabled ? unavailableMessage : '可在设置中重新开启。' }}</p></div>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.keyboard-insights {
  display: grid;
  gap: 20px;
  margin-top: 20px;
}

.keyboard-feature-section,
.keyboard-support-grid {
  min-width: 0;
  padding-top: 20px;
  border-top: 1px solid var(--border-soft);
}

.keyboard-section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.keyboard-heading-copy {
  min-width: 0;
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.keyboard-heading-icon {
  flex: 0 0 auto;
  margin-top: 1px;
  color: var(--accent-green-strong);
}

h2 {
  margin: 0;
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 650;
  letter-spacing: -.2px;
}

p {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 10px;
  line-height: 1.55;
}

.keyboard-status {
  flex: 0 0 auto;
  color: var(--accent-green-strong);
  font-size: 10px;
  font-weight: 600;
}

.keyboard-board {
  display: grid;
  grid-template-columns: minmax(0, 4.5fr) minmax(150px, 1fr) minmax(180px, 1.25fr);
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  background: var(--bg-inset);
}

.keyboard-section {
  min-width: 0;
  display: grid;
  grid-template-rows: 13px minmax(0, 1fr);
  gap: 6px;
}

.keyboard-section > span {
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 650;
  letter-spacing: .08em;
}

.keyboard-section--function { grid-column: 1 / -1; }
.keyboard-rows { display: grid; gap: 5px; }
.keyboard-row { min-width: 0; display: flex; gap: 5px; }
.keyboard-section--function .keyboard-row { gap: 7px; }
.keyboard-section--navigation .keyboard-row:nth-child(3) { justify-content: center; padding-inline: calc(33.333% + 2px); }
.keyboard-section--navigation .keyboard-row:nth-child(4) { justify-content: center; }

.key-cap {
  --heat-fill: var(--bg-soft);
  min-width: 0;
  min-height: 31px;
  flex: var(--key-units) 1 0;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 2px;
  padding: 4px 3px;
  border: 1px solid color-mix(in srgb, var(--accent-green) 14%, var(--border-strong));
  border-bottom-width: 2px;
  border-radius: 6px;
  color: var(--text-secondary);
  background: var(--heat-fill);
  box-shadow: inset 0 1px color-mix(in srgb, var(--text-inverse) 5%, transparent);
  cursor: default;
  transition: transform 160ms var(--ease-out), border-color 160ms ease, filter 160ms ease;
}

.keyboard-section--function .key-cap { min-height: 27px; }

.key-cap.has-activity {
  color: var(--text-primary);
  border-color: color-mix(in srgb, var(--accent-green) 42%, var(--border-strong));
}

.key-cap kbd {
  max-width: 100%;
  overflow: hidden;
  color: currentColor;
  font: 650 var(--text-xs)/1.2 var(--font-ui);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.key-cap small {
  color: var(--text-primary);
  font: 700 var(--text-xs)/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.keyboard-scale {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.keyboard-scale i {
  width: 150px;
  height: 5px;
  border-radius: 99px;
  background: linear-gradient(90deg, var(--bg-soft), color-mix(in srgb, var(--accent-green) 72%, var(--bg-soft)));
}

.keyboard-support-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.18fr) minmax(280px, .82fr);
}

.ranking-section {
  min-width: 0;
  padding-right: 24px;
}

.shortcut-section {
  min-width: 0;
  padding-left: 24px;
  border-left: 1px solid var(--border-soft);
}

.ranking-section ol {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 20px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.ranking-section li {
  min-width: 0;
  display: grid;
  grid-template-columns: 20px 46px minmax(40px, 1fr) auto;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  border-bottom: 1px solid var(--border-soft);
}

.ranking-section li > span {
  color: var(--text-muted);
  font: 600 var(--text-xs)/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.ranking-section li > kbd {
  min-width: 0;
  overflow: hidden;
  color: var(--text-primary);
  font: 650 10px/1.2 var(--font-ui);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ranking-section li > i {
  height: 4px;
  overflow: hidden;
  border-radius: 99px;
  background: var(--bg-soft);
}

.ranking-section li > i em {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent-green);
}

.ranking-section li > strong {
  color: var(--text-secondary);
  font: 650 10px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.shortcut-list {
  display: grid;
}

.shortcut-list > div {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  min-height: 36px;
  border-bottom: 1px solid var(--border-soft);
}

.shortcut-keys {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.shortcut-keys kbd {
  min-width: 28px;
  padding: 4px 6px;
  border: 1px solid var(--border-strong);
  border-bottom-width: 2px;
  border-radius: 6px;
  color: var(--text-primary);
  background: var(--bg-inset);
  font: 650 var(--text-xs)/1 var(--font-ui);
  text-align: center;
}

.shortcut-keys i {
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-style: normal;
}

.shortcut-list strong {
  color: var(--text-secondary);
  font: 650 10px/1 var(--font-data);
  font-variant-numeric: tabular-nums;
}

.detail-empty {
  min-height: 146px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 16px;
  border: 1px dashed var(--border-strong);
  border-radius: 10px;
  color: var(--text-muted);
  background: var(--bg-inset);
}

.detail-empty__icon {
  width: 36px;
  height: 36px;
  flex: 0 0 36px;
  display: grid;
  place-items: center;
  color: var(--accent-green-strong);
}

.detail-empty strong {
  display: block;
  color: var(--text-primary);
  font-size: 11px;
}

.detail-empty p { max-width: 280px; }
.detail-empty--small,
.compact-empty { min-height: 108px; }

.compact-empty {
  display: grid;
  place-items: center;
  color: var(--text-muted);
  font-size: 10px;
}

@media (max-width: 1120px) {
  .keyboard-board { grid-template-columns: minmax(0, 3fr) minmax(140px, .8fr); }
  .keyboard-section--numpad { display: none; }
  .keyboard-support-grid { grid-template-columns: 1fr; }
  .ranking-section { padding-right: 0; }
  .shortcut-section { margin-top: 20px; padding: 20px 0 0; border-top: 1px solid var(--border-soft); border-left: 0; }
  .shortcut-list { grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 20px; }
}

@media (max-width: 760px) {
  .keyboard-board { grid-template-columns: 1fr; }
  .keyboard-section--function { display: none; }
  .keyboard-section--navigation { grid-column: auto; }
  .ranking-section ol,
  .shortcut-list { grid-template-columns: 1fr; }
}
</style>
