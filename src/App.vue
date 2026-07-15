<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import {
  PhBell, PhChartLineUp, PhClockCounterClockwise, PhGearSix, PhHouse,
  PhCopy, PhKeyboard, PhMinus, PhRobot, PhSquare, PhX,
} from '@phosphor-icons/vue'
import { router, pageIds, type PageId } from './router'
import { useAppStore } from './stores/appStore'
import {
  hideWindow, isWindowMaximized, listenDesktop, listenWindowResize, minimizeWindow,
  quitApplication, startWindowDragging, toggleMaximizeWindow,
} from './platform/desktop'
import AiDetailDrawer from './components/AiDetailDrawer.vue'
import CloseDialog from './components/CloseDialog.vue'

const store = useAppStore()
const route = useRoute()
const requestedTheme = new URLSearchParams(window.location.search).get('theme')
const maximized = ref(false)
const navItems = [
  { id: 'home', label: '首页', icon: PhHouse },
  { id: 'ai', label: 'AI 代理', icon: PhRobot },
  { id: 'timeline', label: '时间线', icon: PhClockCounterClockwise },
  { id: 'input', label: '输入足迹', icon: PhKeyboard },
  { id: 'weekly', label: '周报', icon: PhChartLineUp },
  { id: 'goals', label: '提醒与目标', icon: PhBell },
  { id: 'settings', label: '设置', icon: PhGearSix },
] as const
const cleanups: Array<() => void> = []

function isInteractiveTarget(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest('input, textarea, select, button, a, [role="dialog"], [data-shortcuts="ignore"], .detail-drawer, .timeline-board, .chart-card'))
}

function navigateRelative(delta: number): void {
  const current = pageIds.indexOf(route.name as PageId)
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
  if (requestedTheme === 'light' || requestedTheme === 'dark') store.state.theme = requestedTheme
  store.applyTheme()
  await store.setRecording(store.state.recording)
  window.addEventListener('keydown', handleKeydown)
  cleanups.push(await listenDesktop<boolean>('recording-status', (recording) => { store.state.recording = recording }))
  cleanups.push(await listenDesktop<string>('navigate-to', (page) => router.push({ name: page })))
  cleanups.push(await listenDesktop('toggle-reminders', () => { store.state.reminders = !store.state.reminders }))
  cleanups.push(await listenDesktop('native-close-requested', () => requestClose()))
  cleanups.push(await listenWindowResize(() => { void syncMaximized() }))
  await syncMaximized()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  cleanups.forEach((cleanup) => cleanup())
})
</script>

<template>
  <div class="desktop-app">
    <aside class="sidebar">
      <div class="brand-block" @mousedown="handleTitleMouseDown" @dblclick="handleTitleDoubleClick">
        <img src="/src/assets/logo.svg" alt="iTime" />
        <div><strong>iTime</strong><span>v0.1.0</span></div>
      </div>
      <nav aria-label="主导航">
        <RouterLink v-for="item in navItems" :key="item.id" :to="`/${item.id}`" class="nav-item">
          <component :is="item.icon" :size="19" /><span>{{ item.label }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-spacer"></div>
      <div class="profile-card" aria-label="账户与 Pro 入口">
        <img src="/src/assets/avatar-lin.svg" alt="林小北的头像" />
        <div><strong>林小北</strong><small>个人空间</small></div>
        <span class="pro-badge">Pro</span>
      </div>
      <div class="goal-ring-card">
        <div class="goal-ring"><span>75%</span></div>
        <div class="goal-copy"><strong>今日目标</strong><small><span>专注</span><b>6.5 / 8 小时</b></small><small><span>代理</span><b>3.0 / 4 小时</b></small></div>
      </div>
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
          <Transition name="page" mode="out-in"><component :is="Component" /></Transition>
        </RouterView>
      </main>
    </section>
    <AiDetailDrawer />
    <CloseDialog />
    <Transition name="toast"><div v-if="store.state.toast" class="toast">{{ store.state.toast }}</div></Transition>
  </div>
</template>
