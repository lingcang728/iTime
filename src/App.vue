<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import {
  PhChartBar,
  PhCopy,
  PhDatabase,
  PhGearSix,
  PhHouse,
  PhListBullets,
  PhMinus,
  PhMouse,
  PhRobot,
  PhSquare,
  PhTarget,
  PhX,
} from '@phosphor-icons/vue'
import { router, pageIds, type PageId } from './router'
import { useAppStore } from './stores/appStore'
import { hasActivityData } from './stores/dataAvailability'
import {
  hideWindow, isWindowMaximized, listenDesktop, listenWindowResize, minimizeWindow,
  quitApplication, startWindowDragging, toggleMaximizeWindow,
} from './platform/desktop'
import AiDetailDrawer from './components/AiDetailDrawer.vue'
import AppMark from './components/AppMark.vue'
import CloseDialog from './components/CloseDialog.vue'
import packageMetadata from '../package.json'

const store = useAppStore()
const route = useRoute()
const requestedTheme = new URLSearchParams(window.location.search).get('theme')
const maximized = ref(false)
const navItems = [
  { id: 'home', label: '首页', icon: PhHouse },
  { id: 'ai', label: 'AI 代理', icon: PhRobot },
  { id: 'timeline', label: '时间线', icon: PhListBullets },
  { id: 'input', label: '输入足迹', icon: PhMouse },
  { id: 'weekly', label: '周报', icon: PhChartBar },
  { id: 'goals', label: '提醒与目标', icon: PhTarget },
  { id: 'settings', label: '设置', icon: PhGearSix },
] as const
const cleanups: Array<() => void> = []
const learningDuration = computed(() => store.day.value.apps
  .filter((app) => app.category === '学习')
  .reduce((total, app) => total + app.duration, 0))
const developmentDuration = computed(() => store.day.value.apps
  .filter((app) => app.category === '开发')
  .reduce((total, app) => total + app.duration, 0))
const focusDuration = computed(() => learningDuration.value + developmentDuration.value)
const focusTarget = computed(() => (store.state.goals.learning + store.state.goals.development) * 60_000)
const agentDuration = computed(() => store.day.value.aiInteraction.value ?? 0)
const agentTarget = computed(() => store.state.goals.ai * 60_000)
const goalProgress = computed(() => {
  const target = focusTarget.value + agentTarget.value
  return target ? Math.min(100, Math.round((focusDuration.value + agentDuration.value) / target * 100)) : 0
})
const focusProgress = computed(() => focusTarget.value ? Math.min(100, Math.round(focusDuration.value / focusTarget.value * 100)) : 0)
const agentProgress = computed(() => agentTarget.value ? Math.min(100, Math.round(agentDuration.value / agentTarget.value * 100)) : 0)
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const goalProgressLabel = computed(() => activityDataAvailable.value ? `${goalProgress.value}%` : '—')

function formatGoalHours(value: number, target: number): string {
  if (!activityDataAvailable.value) return store.state.activityDataStatus === 'loading' ? '正在读取' : '暂不可用'
  const hours = (value / 3_600_000).toFixed(1)
  const targetHours = (target / 3_600_000).toFixed(1)
  return `${hours} / ${targetHours} 小时`
}

watch(() => route.fullPath, async () => {
  await nextTick()
  document.getElementById('main-content')?.scrollTo({ top: 0, behavior: 'auto' })
})

function isInteractiveTarget(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest('input, textarea, select, button, a, [role="dialog"], [data-shortcuts="ignore"], .detail-drawer, .timeline-board, .chart-card'))
}

function navigateRelative(delta: number): void {
  const current = pageIds.indexOf(route.name as PageId)
  if (current < 0) return
  const next = Math.max(0, Math.min(pageIds.length - 1, current + delta))
  if (next !== current) router.push({ name: pageIds[next] })
}

function handleKeydown(event: KeyboardEvent): void {
  if (isInteractiveTarget(event.target) || store.state.closeDialogOpen || store.state.detailDrawerOpen) return
  if (event.ctrlKey && event.key === 'PageUp') { event.preventDefault(); navigateRelative(-1); return }
  if (event.ctrlKey && event.key === 'PageDown') { event.preventDefault(); navigateRelative(1); return }
  if (event.altKey && event.key === 'ArrowLeft') { event.preventDefault(); router.back(); return }
  if (event.altKey && event.key === 'ArrowRight') { event.preventDefault(); router.forward(); return }
  if (!event.altKey && !event.ctrlKey && !event.metaKey && event.target === document.body && event.key === 'ArrowLeft') navigateRelative(-1)
  if (!event.altKey && !event.ctrlKey && !event.metaKey && event.target === document.body && event.key === 'ArrowRight') navigateRelative(1)
}

async function requestClose(): Promise<void> {
  if (store.state.closePreference === 'hide') { await hideWindow(); return }
  if (store.state.closePreference === 'quit') { await quitApplication(); return }
  store.state.closeDialogOpen = true
}

async function syncMaximized(): Promise<void> {
  maximized.value = await isWindowMaximized()
}

async function handleTitleMouseDown(event: MouseEvent): Promise<void> {
  if (event.button !== 0 || event.detail > 1) return
  if (event.target instanceof Element && event.target.closest('.window-controls')) return
  await startWindowDragging()
  await syncMaximized()
}

async function handleTitleDoubleClick(event: MouseEvent): Promise<void> {
  if (event.target instanceof Element && event.target.closest('.window-controls')) return
  await toggleMaximizeWindow()
  await syncMaximized()
}

async function toggleWindowSize(): Promise<void> {
  await toggleMaximizeWindow()
  await syncMaximized()
}

onMounted(async () => {
  store.applyTheme(requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined)
  window.addEventListener('keydown', handleKeydown)
  cleanups.push(...await Promise.all([
    listenDesktop<boolean>('recording-status', (recording) => { store.state.recording = recording }),
    listenDesktop<string>('recording-error', (message) => store.showToast(message)),
    listenDesktop<string>('navigate-to', (page) => router.push({ name: page })),
    listenDesktop('toggle-reminders', () => { store.state.reminders = !store.state.reminders }),
    listenDesktop('native-close-requested', () => requestClose()),
    listenWindowResize(() => { void syncMaximized() }),
  ]))
  await store.syncRecording()
  await syncMaximized()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  cleanups.forEach((cleanup) => {
    cleanup()
  })
})
</script>

<template>
  <div class="desktop-app">
    <aside class="sidebar">
      <div class="brand-block" @mousedown="handleTitleMouseDown" @dblclick="handleTitleDoubleClick">
        <AppMark :size="34" />
        <div><strong>iTime</strong><span>v{{ packageMetadata.version }}</span></div>
      </div>
      <nav aria-label="主导航">
        <RouterLink v-for="item in navItems" :key="item.id" :to="`/${item.id}`" class="nav-item">
          <component :is="item.icon" class="nav-icon" :size="20" weight="regular" aria-hidden="true" /><span>{{ item.label }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-spacer"></div>
      <button class="profile-card" type="button" aria-label="打开本机数据设置" @click="router.push({ name: 'settings' })">
        <PhDatabase :size="22" weight="regular" aria-hidden="true" />
        <div><strong>本机数据</strong><small>仅保存在这台电脑</small></div>
      </button>
      <button class="goal-ring-card" type="button" aria-label="查看提醒与目标" @click="router.push({ name: 'goals' })">
        <div class="goal-copy__header"><strong>今日目标</strong><b>{{ goalProgressLabel }}</b></div>
        <div class="goal-copy">
          <small><span>专注</span><b>{{ formatGoalHours(focusDuration, focusTarget) }}</b></small>
          <span class="goal-meter"><i :style="{ width: `${activityDataAvailable ? focusProgress : 0}%` }"></i></span>
          <small><span>AI 前台</span><b>{{ formatGoalHours(agentDuration, agentTarget) }}</b></small>
          <span class="goal-meter"><i :style="{ width: `${activityDataAvailable ? agentProgress : 0}%` }"></i></span>
        </div>
      </button>
    </aside>
    <section class="app-surface">
      <div class="window-bar" @mousedown="handleTitleMouseDown" @dblclick="handleTitleDoubleClick">
        <span aria-hidden="true"></span>
        <div class="window-controls" data-shortcuts="ignore">
          <button type="button" aria-label="最小化" @click="minimizeWindow"><PhMinus :size="14" /></button>
          <button type="button" :aria-label="maximized ? '还原' : '最大化'" @click="toggleWindowSize">
            <PhCopy v-if="maximized" :size="12" />
            <PhSquare v-else :size="12" />
          </button>
          <button type="button" aria-label="关闭" class="close" @click="requestClose"><PhX :size="14" /></button>
        </div>
      </div>
      <main id="main-content" class="page-viewport" tabindex="-1">
        <RouterView v-slot="{ Component }">
          <Transition name="page"><component :is="Component" /></Transition>
        </RouterView>
      </main>
    </section>
    <AiDetailDrawer />
    <CloseDialog />
    <Transition name="toast"><div v-if="store.state.toast" class="toast" role="status" aria-live="polite">{{ store.state.toast }}</div></Transition>
  </div>
</template>

<style scoped src="./styles/sidebar-account.css"></style>
