import { createRouter, createWebHashHistory } from 'vue-router'
import HomePage from './pages/HomePage.vue'

export const pageIds = ['home', 'ai', 'timeline', 'input', 'weekly', 'goals', 'settings'] as const
export type PageId = typeof pageIds[number]

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/home' },
    { path: '/home', name: 'home', component: HomePage },
    { path: '/ai', name: 'ai', component: () => import('./pages/AiAgentsPage.vue') },
    { path: '/timeline', name: 'timeline', component: () => import('./pages/TimelinePage.vue') },
    { path: '/input', name: 'input', component: () => import('./pages/InputFootprintPage.vue') },
    { path: '/weekly', name: 'weekly', component: () => import('./pages/WeeklyPage.vue') },
    { path: '/goals', name: 'goals', component: () => import('./pages/GoalsPage.vue') },
    { path: '/settings', name: 'settings', component: () => import('./pages/SettingsPage.vue') },
    { path: '/:pathMatch(.*)*', redirect: '/home' },
  ],
})

// After an in-place update the running webview can reference chunks that no
// longer exist; a failed lazy import would otherwise leave a blank page.
const CHUNK_RELOAD_FLAG = 'itime:chunk-reload-attempted'
const chunkErrorHints = [
  'dynamically imported module',
  'error loading dynamically imported module',
  'importing a module script failed',
  'failed to fetch',
  'chunkloaderror',
]
let reloadMarkedInMemory = false
function reloadAttempted(mark: boolean): boolean {
  let flagged = reloadMarkedInMemory
  try {
    if (mark) sessionStorage.setItem(CHUNK_RELOAD_FLAG, '1')
    flagged ||= Boolean(sessionStorage.getItem(CHUNK_RELOAD_FLAG))
  } catch { /* sessionStorage may be unavailable */ }
  if (mark) reloadMarkedInMemory = true
  return flagged
}

router.onError((error) => {
  const message = String((error as Error)?.message ?? error).toLowerCase()
  if (!chunkErrorHints.some((hint) => message.includes(hint))) {
    console.error('[router] navigation failed', error)
    return
  }
  if (reloadAttempted(false)) {
    // Already reloaded once this session and chunks still fail — don't loop.
    console.error('[router] chunk still missing after reload', error)
    return
  }
  reloadAttempted(true)
  void import('./stores/appStore')
    .then(({ useAppStore }) => useAppStore().showToast('应用已更新，正在重新载入…'))
    .catch(() => undefined)
    .finally(() => window.location.reload())
})
