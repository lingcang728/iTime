<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import {
  PhBell,
  PhCaretRight,
  PhClock,
  PhCloudCheck,
  PhCopy,
  PhGearSix,
  PhHouse,
  PhKeyboard,
  PhMinus,
  PhNotebook,
  PhPause,
  PhShieldCheck,
  PhSparkle,
  PhSquare,
  PhX,
} from '@phosphor-icons/vue'
import { router, pageIds, type PageId } from './router'
import { useAppStore } from './stores/appStore'
import {
  configureDesktopReminders,
  hideWindow, isTauriRuntime, isWindowMaximized, listenDesktop, listenWindowResize, minimizeWindow,
  quitApplication, startWindowDragging, toggleMaximizeWindow,
} from './platform/desktop'
import AiDetailDrawer from './components/AiDetailDrawer.vue'
import AppMark from './components/AppMark.vue'
import CloseDialog from './components/CloseDialog.vue'
import RecordingPausedBanner from './components/RecordingPausedBanner.vue'
import { useNow } from './composables/useNow'
import { runtimeSyncStatus } from './stores/runtimeStatus'
import { registerListenersIndependently } from './platform/listenerRegistry'
import { checkForDesktopUpdate } from './services/updateService'

const store = useAppStore()
const { nowMs } = useNow()
const route = useRoute()
const requestedTheme = new URLSearchParams(window.location.search).get('theme')
const maximized = ref(false)
// First-run collection notice (P2-30): the app starts recording on first
// launch, so the privacy disclosure must appear without requiring a visit to
// Settings. Persisted locally; it is a UI disclosure flag, not a data record.
const FIRST_RUN_NOTICE_KEY = 'itime:first-run-notice-seen:v1'
// localStorage 访问可能抛异常（隐私模式/配额）——提示属尽力而为，不能拖垮入口。
function readFirstRunSeen(): boolean {
  try { return window.localStorage.getItem(FIRST_RUN_NOTICE_KEY) === '1' } catch { return true }
}
const showFirstRunNotice = ref(isTauriRuntime() && !readFirstRunSeen())
function dismissFirstRunNotice(): void {
  try { window.localStorage.setItem(FIRST_RUN_NOTICE_KEY, '1') } catch { /* best effort */ }
  showFirstRunNotice.value = false
}
const navItems = [
  { id: 'home', label: '首页', icon: PhHouse },
  { id: 'ai', label: 'AI 工具', icon: PhSparkle },
  { id: 'timeline', label: '时间线', icon: PhClock },
  { id: 'input', label: '键盘统计', icon: PhKeyboard },
  { id: 'weekly', label: '周报', icon: PhNotebook },
  { id: 'goals', label: '提醒与目标', icon: PhBell },
  { id: 'settings', label: '设置', icon: PhGearSix },
] as const
// F-15: navItems 与 router.ts 的 pageIds 双份维护——开发期对漂移直接报警。
if (import.meta.env.DEV
  && (navItems.length !== pageIds.length || navItems.some((item) => !(pageIds as readonly string[]).includes(item.id)))) {
  console.error('[App] navItems 与 router pageIds 不一致', { navItems, pageIds })
}
const cleanups: Array<() => void> = []
let reminderSync = Promise.resolve()
const syncStatus = computed(() => runtimeSyncStatus({
  desktop: isTauriRuntime(),
  now: nowMs.value,
  lastUpdatedAt: store.state.lastDataRefreshAt,
  recordingPaused: !store.state.recording,
  statuses: [
    store.state.activityDataStatus,
    store.state.inputDataStatus,
    store.state.providerDataStatus,
  ],
  messages: [
    store.state.activityDataMessage,
    store.state.inputDataMessage,
    store.state.providerDataMessage,
  ],
}))

// 记录暂停/切换中与数据同步状态分开呈现：暂停是用户可感知的一等功能状态。
const syncDisplay = computed(() => {
  if (store.state.recordingStatus === 'loading') {
    return { state: 'busy', title: '记录状态切换中', detail: store.state.recordingMessage }
  }
  if (store.state.recordingStatus === 'error') {
    return { state: 'error', title: '记录状态异常', detail: store.state.recordingMessage }
  }
  if (!store.state.recording) {
    return { state: 'paused', title: '记录已暂停', detail: '新活动不会写入；点这里前往设置恢复' }
  }
  return syncStatus.value
})

watch(() => route.fullPath, async () => {
  await nextTick()
  const main = document.getElementById('main-content')
  main?.scrollTo({ top: 0, behavior: 'auto' })
  // SPA 路由切换后把焦点交给主内容区，让读屏器报出新页面。
  main?.focus({ preventScroll: true })
})

function isInteractiveTarget(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest('input, textarea, select, button, a, [role="dialog"], [data-shortcuts="ignore"]'))
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
  if (store.state.closePreference === 'hide') {
    // 与 CloseDialog.choose 同一约定：失败必须可见，不能静默吞掉。
    try { await hideWindow() } catch (error) {
      store.showToast(error instanceof Error ? error.message : String(error), 'error')
    }
    return
  }
  if (store.state.closePreference === 'quit') {
    try { await quitApplication() } catch (error) {
      store.showToast(error instanceof Error ? error.message : String(error), 'error')
    }
    return
  }
  store.state.closeDialogOpen = true
}

// P2-22: resize 事件在拖拽期间高频触发——合并并发调用，保证最终状态收敛。
let maximizeSyncing = false
let maximizeSyncDirty = false
async function syncMaximized(): Promise<void> {
  if (maximizeSyncing) { maximizeSyncDirty = true; return }
  maximizeSyncing = true
  try {
    do {
      maximizeSyncDirty = false
      try { maximized.value = await isWindowMaximized() } catch { /* 保留上次已知状态 */ }
    } while (maximizeSyncDirty)
  } finally {
    maximizeSyncing = false
  }
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
    .then(() => {
      store.state.reminderSyncFailed = false
      store.state.reminderSyncMessage = ''
    })
    .catch((error) => {
      const message = error instanceof Error ? error.message : String(error)
      // P3-26: 推送失败后端配置与开关可能不一致——保留可见状态供提醒面板展示。
      store.state.reminderSyncFailed = true
      store.state.reminderSyncMessage = message
      store.showToast(`休息提醒暂不可用：${message}`, 'error')
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

// P2-22: listen() 是异步的——组件若在注册途中卸载，迟到的 unlisten 不能再丢。
let listenersDisposed = false
onMounted(async () => {
  store.applyTheme(requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined)
  await nextTick()
  void checkForDesktopUpdate(false)
  window.addEventListener('keydown', handleKeydown)
  const listenerError = (error: unknown) => {
    const message = error instanceof Error ? error.message : String(error)
    store.showToast(`桌面事件监听暂不可用：${message}`, 'error')
  }
  const registered = await registerListenersIndependently([
    () => listenDesktop<boolean>('recording-status', (recording) => {
      store.state.recording = recording
      store.state.recordingStatus = 'ready'
      store.state.recordingMessage = recording ? '记录中' : '已暂停'
    }),
    () => listenDesktop<string>('recording-error', (message) => {
      store.state.recordingStatus = 'error'
      store.state.recordingMessage = message
      store.showToast(message, 'error')
    }),
    () => listenDesktop<string>('navigate-to', (page) => router.push({ name: page })),
    // P2-27: 托盘切换提醒要给即时反馈，配置推送失败由 syncReminderConfiguration 另报。
    () => listenDesktop('toggle-reminders', () => {
      store.state.reminders = !store.state.reminders
      store.showToast(store.state.reminders ? '休息提醒已开启' : '休息提醒已关闭')
    }),
    () => listenDesktop<{ occurrenceId: string; continuousMinutes: number }>('rest-reminder-due', (occurrence) => {
      if (store.receiveReminder(occurrence)) {
        store.showToast(`休息提醒：已连续使用 ${occurrence.continuousMinutes} 分钟`)
      }
    }),
    () => listenDesktop<string>('rest-reminder-error', (message) => {
      store.showToast(`系统通知发送失败：${message}`, 'error')
    }),
    () => listenDesktop('native-close-requested', () => requestClose()),
    () => listenWindowResize(() => { void syncMaximized() }),
  ], listenerError)
  if (listenersDisposed) {
    registered.forEach((cleanup) => cleanup())
    return
  }
  cleanups.push(...registered)
  await store.syncRecording()
  await syncMaximized()
})

onBeforeUnmount(() => {
  listenersDisposed = true
  window.removeEventListener('keydown', handleKeydown)
  cleanups.forEach((cleanup) => {
    cleanup()
  })
})
</script>

<template>
  <div class="desktop-app" :data-maximized="maximized">
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
      <button class="profile-card sync-status" :data-state="syncDisplay.state" type="button" :aria-label="`${syncDisplay.title}，${syncDisplay.detail}；打开本机数据设置`" @click="router.push({ name: 'settings' })">
        <PhPause v-if="syncDisplay.state === 'paused'" :size="24" weight="regular" aria-hidden="true" />
        <PhCloudCheck v-else :size="24" weight="regular" aria-hidden="true" />
        <div><strong>{{ syncDisplay.title }}</strong><small>{{ syncDisplay.detail }}</small></div>
        <PhCaretRight class="sync-status__chevron" :size="14" aria-hidden="true" />
      </button>
    </aside>
    <section class="app-surface">
      <div class="window-bar" @mousedown="handleTitleMouseDown" @dblclick="handleTitleDoubleClick">
        <span aria-hidden="true"></span>
        <div class="window-controls" data-shortcuts="ignore">
          <button type="button" aria-label="最小化" @click="minimizeWindow"><PhMinus :size="14" aria-hidden="true" /></button>
          <button type="button" :aria-label="maximized ? '还原' : '最大化'" @click="toggleWindowSize">
            <PhCopy v-if="maximized" :size="12" aria-hidden="true" />
            <PhSquare v-else :size="12" aria-hidden="true" />
          </button>
          <button type="button" aria-label="关闭" class="close" @click="requestClose"><PhX :size="14" aria-hidden="true" /></button>
        </div>
      </div>
      <div v-if="showFirstRunNotice" class="first-run-notice" role="note">
        <PhShieldCheck :size="18" aria-hidden="true" />
        <p>iTime 正在本机记录前台应用与按键计数（不记录任何内容）；数据只保存在本机，可在「设置 → 本地数据」随时导出或删除。</p>
        <button type="button" @click="dismissFirstRunNotice">知道了</button>
      </div>
      <main id="main-content" class="page-viewport" tabindex="-1">
        <!-- P1-12: 暂停/切换中状态是所有数据页的全局状态，挂在页面壳上而非逐页重复。
             .page-banner-slot 自带与 .page 相同的宽度与断点侧距；:empty 时收起不占空间。 -->
        <div class="page-banner-slot"><RecordingPausedBanner /></div>
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in"><component :is="Component" /></Transition>
        </RouterView>
      </main>
    </section>
    <AiDetailDrawer />
    <CloseDialog />
    <Transition name="toast"><div v-if="store.state.toast" class="toast" :data-tone="store.state.toastTone" :role="store.state.toastTone === 'error' ? 'alert' : 'status'">{{ store.state.toast }}</div></Transition>
  </div>
</template>
