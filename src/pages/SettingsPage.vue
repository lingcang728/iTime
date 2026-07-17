<script setup lang="ts">
import { computed, onMounted } from 'vue'
import {
  PhArrowClockwise,
  PhDesktop,
  PhMoon,
  PhPauseCircle,
  PhShieldCheck,
  PhSun,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { uiIcons } from '../data/uiIcons'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()

const inputStatusLabel = computed(() => ({
  loading: '正在连接',
  preview: '预览数据',
  ready: '本机已连接',
  unavailable: '暂时不可用',
}[store.state.inputDataStatus]))

const autostartStatusLabel = computed(() => ({
  loading: '正在向 Windows 确认',
  ready: store.state.autostartMessage || (store.state.autostartEnabled ? '系统启动项已启用' : '系统启动项未启用'),
  error: store.state.autostartMessage || '无法读取系统启动项',
}[store.state.autostartStatus]))

const keyStatsFacts = computed(() => {
  const capabilities = store.input.value.capabilities
  const historyLabel = {
    minute: '分钟级',
    hour: '小时级',
    day: '日级',
    none: '无历史序列',
  }[capabilities.historyGranularity]
  return [
    { label: '访问方式', value: capabilities.deleteByDate ? '由数据源授权读写' : '只读，不修改 KeyStats' },
    { label: '历史粒度', value: historyLabel },
    { label: '键位统计', value: capabilities.keyHeatmap ? '可用' : '数据源未提供' },
    { label: '功能组合键', value: capabilities.functionalShortcuts ? '可用' : '数据源未提供' },
    { label: '历史左右键', value: capabilities.splitHistoricalClicks ? '可分别统计' : '仅提供合计' },
    { label: '日期口径', value: capabilities.timezoneSemantics === 'utc-date-bucket' ? 'KeyStats UTC 日期桶' : '本地自然日' },
  ]
})

function checkedValue(event: Event): boolean {
  return event.currentTarget instanceof HTMLInputElement && event.currentTarget.checked
}

function updateClosePreference(event: Event): void {
  store.state.closePreference = checkedValue(event) ? 'hide' : 'ask'
}

async function updateAutostart(event: Event): Promise<void> {
  await store.setAutostart(checkedValue(event))
}

onMounted(() => void store.refreshAutostart())
</script>

<template>
  <section class="page settings-page">
    <PageHeader title="设置" subtitle="管理启动方式、记录范围、提醒与本地数据" />
    <div class="settings-layout">
      <main class="settings-main">
        <article class="card settings-section">
          <header class="section-heading">
            <span class="section-icon blue section-icon--art"><img :src="uiIcons.pageSettings" alt="" draggable="false" /></span>
            <div><h2>启动与窗口</h2><p>这些选项会影响 iTime 在 Windows 中的实际运行方式。</p></div>
          </header>
          <label class="control-row">
            <div><strong>开机自启动</strong><span>登录 Windows 后自动启动 iTime，不必手动打开。</span><small :class="['system-status', store.state.autostartStatus]">{{ autostartStatusLabel }}</small></div>
            <span class="toggle"><input :checked="store.state.autostartEnabled" :disabled="store.state.autostartStatus === 'loading'" type="checkbox" @change="updateAutostart"><i></i></span>
          </label>
          <label class="control-row">
            <div><strong>关闭窗口时隐藏到托盘</strong><span>保留本机记录进程，可从托盘再次打开。</span></div>
            <span class="toggle"><input :checked="store.state.closePreference === 'hide'" type="checkbox" @change="updateClosePreference"><i></i></span>
          </label>
          <label class="control-row">
            <div><strong>活动记录</strong><span>{{ store.state.recording ? '正在记录启用后的本机活动' : '当前已暂停记录' }}</span></div>
            <span class="toggle"><input :checked="store.state.recording" type="checkbox" @change="store.setRecording(!store.state.recording)"><i></i></span>
          </label>
        </article>

        <article class="card settings-section">
          <header class="section-heading">
            <span class="section-icon green section-icon--art"><img :src="uiIcons.inputKeystrokes" alt="" draggable="false" /></span>
            <div><h2>输入统计与隐私</h2><p>只保存聚合计数，不保存输入内容或可还原文字的事件序列。</p></div>
          </header>
          <label class="control-row"><div><strong>键盘热力图</strong><span>显示数据源提供的单键累计次数。</span></div><span class="toggle"><input v-model="store.state.heatmapEnabled" type="checkbox"><i></i></span></label>
          <label class="control-row"><div><strong>功能组合键</strong><span>仅显示 Ctrl+C 等明确功能组合的累计次数。</span></div><span class="toggle"><input v-model="store.state.shortcutsEnabled" type="checkbox"><i></i></span></label>
          <div class="privacy-note"><PhShieldCheck :size="23" weight="duotone" /><p>iTime 不读取键盘文字、密码内容或剪贴板正文。KeyStats 数据通过本机文件只读接入，删除操作不会反向修改 KeyStats。</p></div>
        </article>

        <article class="card settings-section appearance-section">
          <header class="section-heading">
            <span class="section-icon violet section-icon--art"><img :src="uiIcons.metricComputer" alt="" draggable="false" /></span>
            <div><h2>外观</h2><p>选择适合当前 Windows 桌面的显示方式。</p></div>
          </header>
          <div class="theme-options" role="radiogroup" aria-label="主题">
            <label :class="{ active: store.state.theme === 'light' }"><input v-model="store.state.theme" type="radio" value="light"><PhSun :size="20" /><span>浅色</span></label>
            <label :class="{ active: store.state.theme === 'dark' }"><input v-model="store.state.theme" type="radio" value="dark"><PhMoon :size="20" /><span>深色</span></label>
            <label :class="{ active: store.state.theme === 'system' }"><input v-model="store.state.theme" type="radio" value="system"><PhDesktop :size="20" /><span>跟随系统</span></label>
          </div>
        </article>
      </main>

      <aside class="settings-side">
        <article class="card source-card">
          <header class="section-heading">
            <span class="section-icon orange section-icon--art"><img :src="uiIcons.inputDataSource" alt="" draggable="false" /></span>
            <div><h2>KeyStats 本机数据</h2><p>展示当前实际连接状态与数据能力。</p></div>
          </header>
          <div :class="['source-status', store.state.inputDataStatus]">
            <span class="status-dot"></span><div><strong>{{ inputStatusLabel }}</strong><p>{{ store.state.inputDataMessage }}</p></div>
          </div>
          <dl class="source-facts">
            <div><dt>来源</dt><dd>{{ store.input.value.source }}</dd></div>
            <div v-for="fact in keyStatsFacts" :key="fact.label"><dt>{{ fact.label }}</dt><dd>{{ fact.value }}</dd></div>
          </dl>
          <button class="refresh-button" type="button" :disabled="store.state.inputDataStatus === 'loading'" @click="store.refreshInputData">
            <PhArrowClockwise :size="17" />重新读取本机记录
          </button>
        </article>

        <article class="card data-boundary-card">
          <PhPauseCircle :size="26" weight="duotone" />
          <div><span>数据边界</span><h2>接入前历史不会被补造</h2><p>KeyStats 提供输入聚合；应用与 AI 活动仅从 iTime 采集器启用后开始记录。</p></div>
        </article>
      </aside>
    </div>
  </section>
</template>

<style scoped src="./settings-page.css"></style>
