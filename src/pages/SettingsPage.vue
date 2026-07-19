<script setup lang="ts">
import { computed, onMounted } from 'vue'
import {
  PhArrowClockwise,
  PhChartBar,
  PhClock,
  PhDesktop,
  PhHardDrives,
  PhKeyboard,
  PhMonitor,
  PhMoon,
  PhPalette,
  PhPauseCircle,
  PhPower,
  PhPulse,
  PhShieldCheck,
  PhSun,
  PhTray,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
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

const inputFacts = computed(() => {
  const capabilities = store.input.value.capabilities
  const historyLabel = {
    minute: '分钟级',
    hour: '小时级',
    day: '日级',
    none: '无历史序列',
  }[capabilities.historyGranularity]
  return [
    { label: '总量粒度', value: historyLabel, icon: PhPulse },
    { label: '键位与快捷键', value: capabilities.keyIdentityCaptured ? '按本地日期累计' : '升级后启用', icon: PhKeyboard },
    { label: '顺序与逐键时间', value: '从不保存', icon: PhShieldCheck },
    { label: '本地存储', value: 'JSONL + 原子 JSON', icon: PhHardDrives },
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
    <PageHeader title="设置" subtitle="自定义 iTime 的行为与外观，管理输入统计与隐私设置。" />
    <div class="settings-layout">
      <div class="settings-main">
        <section class="settings-group" aria-labelledby="startup-title">
          <header class="settings-group__header">
            <div><h2 id="startup-title">启动与窗口</h2><p>这些选项会影响 iTime 在 Windows 中的实际运行方式。</p></div>
          </header>
          <div class="settings-list">
            <label class="control-row">
              <span class="control-icon"><PhPower :size="20" /></span>
              <div><strong>开机自启动</strong><span>登录 Windows 后自动启动 iTime，不必手动打开。</span><small :class="['system-status', store.state.autostartStatus]">{{ autostartStatusLabel }}</small></div>
              <span class="toggle"><input :checked="store.state.autostartEnabled" :disabled="store.state.autostartStatus === 'loading'" type="checkbox" @change="updateAutostart"><i></i></span>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhTray :size="20" /></span>
              <div><strong>关闭窗口时隐藏到托盘</strong><span>保留本机记录进程，可从托盘再次打开。</span></div>
              <span class="toggle"><input :checked="store.state.closePreference === 'hide'" type="checkbox" @change="updateClosePreference"><i></i></span>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhChartBar :size="20" /></span>
              <div><strong>活动记录</strong><span>{{ store.state.recording ? '正在记录启用后的本机活动' : '当前已暂停记录' }}</span></div>
              <span class="toggle"><input :checked="store.state.recording" type="checkbox" @change="store.setRecording(!store.state.recording)"><i></i></span>
            </label>
          </div>
        </section>

        <section class="settings-group" aria-labelledby="privacy-title">
          <header class="settings-group__header">
            <div><h2 id="privacy-title">输入统计与隐私</h2><p>分钟总量与日级键位/快捷键聚合都只保存在本机。</p></div>
          </header>
          <div class="privacy-note"><PhShieldCheck :size="20" weight="regular" aria-hidden="true" /><p>iTime 会按分钟保存字符键总量，并按本地日期保存规范化键位与快捷键计数；不会保存按键顺序、逐键时间戳、原始虚拟键码、输入文本、IME 结果或剪贴板内容。</p></div>
        </section>

        <section class="settings-group appearance-section" aria-labelledby="appearance-title">
          <header class="settings-group__header">
            <div><h2 id="appearance-title">外观</h2><p>选择适合当前 Windows 桌面的显示方式。</p></div>
          </header>
          <div class="theme-options" role="radiogroup" aria-label="主题">
            <span class="theme-icon"><PhPalette :size="20" /><i><strong>主题</strong><small>选择 iTime 的外观主题</small></i></span>
            <label :class="{ active: store.state.theme === 'light' }"><input v-model="store.state.theme" type="radio" value="light"><PhSun :size="18" weight="regular" /><span>浅色</span></label>
            <label :class="{ active: store.state.theme === 'dark' }"><input v-model="store.state.theme" type="radio" value="dark"><PhMoon :size="18" weight="regular" /><span>深色</span></label>
            <label :class="{ active: store.state.theme === 'system' }"><input v-model="store.state.theme" type="radio" value="system"><PhDesktop :size="18" weight="regular" /><span>跟随系统</span></label>
          </div>
        </section>
      </div>

      <aside class="settings-side">
        <section class="source-panel" aria-labelledby="source-title">
          <header class="settings-group__header">
            <div><h2 id="source-title">本机输入足迹</h2><p>查看分钟总量和日级键位聚合的当前状态。</p></div>
          </header>
          <div :class="['source-status', store.state.inputDataStatus]">
            <span class="status-dot"></span><div><strong>{{ inputStatusLabel }}</strong><p>{{ store.state.inputDataMessage }}</p></div>
          </div>
          <dl class="source-facts">
            <div v-for="fact in inputFacts" :key="fact.label"><component :is="fact.icon" :size="19" /><dt>{{ fact.label }}</dt><dd>{{ fact.value }}</dd></div>
          </dl>
          <button class="refresh-button" type="button" :disabled="store.state.inputDataStatus === 'loading'" @click="store.refreshInputData">
            <PhArrowClockwise :size="17" weight="regular" />刷新输入足迹
          </button>
        </section>

        <section class="data-boundary">
          <PhPauseCircle :size="22" weight="regular" aria-hidden="true" />
          <div><span>数据边界</span><h2>升级前逐键历史不会被补造</h2><p>旧分钟总量继续用于趋势；键位热力与快捷键排行从本次升级后开始。活动记录总开关会同时暂停两类输入统计。</p></div>
        </section>
      </aside>
    </div>
  </section>
</template>

<style scoped src="./settings-page.css"></style>
