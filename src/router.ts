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
