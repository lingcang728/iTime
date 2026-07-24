<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import {
  PhBell,
  PhClock,
  PhCloudCheck,
  PhCopy,
  PhGearSix,
  PhHouse,
  PhKeyboard,
  PhMinus,
  PhNotebook,
  PhSparkle,
  PhSquare,
  PhX,
} from '@phosphor-icons/vue'
import { router, pageIds, type PageId } from './router'
import { useAppStore } from './stores/appStore'
import {
  configureDesktopReminders,
  hideWindow, isWindowMaximized, listenDesktop, listenWindowResize, minimizeWindow,
  quitApplication, startWindowDragging, toggleMaximizeWindow,
} from './platform/desktop'
import AiDetailDrawer from './components/AiDetailDrawer.vue'
import AppMark from './components/AppMark.vue'
import CloseDialog from './components/CloseDialog.vue'

const store = useAppStore()
const route = useRoute()
const requestedTheme = new URLSearchParams(window.location.search).get('theme')
const maximized = ref(false)
const navItems = [
  { id: 'home', label: '首页', icon: PhHouse },
  { id: 'ai', label: 'AI 代理', icon: PhSparkle },
  { id: 'timeline', label: '时间线', icon: PhClock },
  { id: 'input', label: '键盘统计', icon: PhKeyboard },
  { id: 'weekly', label: '周报', icon: PhNotebook },
  { id: 'goals', label: '提醒与目标', icon: PhBell },
  { id: 'settings', label: '设置', icon: PhGearSix },
] as const
const cleanups: Array<() => void> = []
let reminderSync = Promise.resolve()

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

function syncReminderConfiguration(): void {
  reminderSync = reminderSync
    .catch(() => undefined)
    .then(() => configureDesktopReminders({
      enabled: store.state.reminders,
      intervalMinutes: store.state.goals.continuous,
      quietStart: store.state.quietStart,
      quietEnd: store.state.quietEnd,
    }))
    .catch((error) => {
      const message = error instanceof Error ? error.message : String(error)
      store.showToast(`休息提醒暂不可用：${message}`)
    })
}

watch(
  () => [
    store.state.reminders,
    store.state.goals.continuous,
    store.state.quietStart,
    store.state.quietEnd,
  ],
  syncReminderConfiguration,
  { immediate: true },
)

onMounted(async () => {
  store.applyTheme(requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined)
  window.addEventListener('keydown', handleKeydown)
  cleanups.push(...await Promise.all([
    listenDesktop<boolean>('recording-status', (recording) => { store.state.recording = recording }),
    listenDesktop<string>('recording-error', (message) => store.showToast(message)),
    listenDesktop<string>('navigate-to', (page) => router.push({ name: page })),
    listenDesktop('toggle-reminders', () => { store.state.reminders = !store.state.reminders }),
    listenDesktop<{ continuousMinutes: number }>('rest-reminder-due', ({ continuousMinutes }) => {
      store.showToast(`休息提醒：已连续使用 ${continuousMinutes} 分钟`)
    }),
    listenDesktop<string>('rest-reminder-error', (message) => {
      store.showToast(`系统通知发送失败：${message}`)
    }),
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
        <div><strong>iTime</strong></div>
      </div>
      <nav aria-label="主导航">
        <RouterLink v-for="item in navItems" :key="item.id" :to="`/${item.id}`" class="nav-item">
          <component :is="item.icon" class="nav-icon" :size="20" weight="regular" aria-hidden="true" /><span>{{ item.label }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-spacer"></div>
      <button class="profile-card sync-status" type="button" aria-label="打开本机数据设置" @click="router.push({ name: 'settings' })">
        <PhCloudCheck :size="24" weight="regular" aria-hidden="true" />
        <div><strong>数据已同步</strong><small>刚刚</small></div>
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
