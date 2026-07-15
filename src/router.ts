import { createRouter, createWebHashHistory } from 'vue-router'
import HomePage from './pages/HomePage.vue'
import AiAgentsPage from './pages/AiAgentsPage.vue'
import WeeklyPage from './pages/WeeklyPage.vue'
import TimelinePage from './pages/TimelinePage.vue'
import InputFootprintPage from './pages/InputFootprintPage.vue'
import GoalsPage from './pages/GoalsPage.vue'
import SettingsPage from './pages/SettingsPage.vue'

export const pageIds = ['home', 'ai', 'timeline', 'input', 'weekly', 'goals', 'settings'] as const
export type PageId = typeof pageIds[number]

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/home' },
    { path: '/home', name: 'home', component: HomePage },
    { path: '/ai', name: 'ai', component: AiAgentsPage },
    { path: '/timeline', name: 'timeline', component: TimelinePage },
    { path: '/input', name: 'input', component: InputFootprintPage },
    { path: '/weekly', name: 'weekly', component: WeeklyPage },
    { path: '/goals', name: 'goals', component: GoalsPage },
    { path: '/settings', name: 'settings', component: SettingsPage },
  ],
})
