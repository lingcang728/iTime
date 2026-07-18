import { createApp } from 'vue'
import App from './App.vue'
import { router } from './router'
import { loadPersistedState } from './stores/persistedState'
import { applyDocumentTheme, resolveTheme, systemPrefersDark, type ResolvedTheme } from './stores/theme'
import './styles/tokens.css'
import './styles/global.css'

const query = new URLSearchParams(window.location.search)
if (query.get('reference') === 'compact') document.documentElement.dataset.reference = 'compact'
const requestedTheme = query.get('theme')
const previewTheme: ResolvedTheme | undefined = requestedTheme === 'light' || requestedTheme === 'dark' ? requestedTheme : undefined
const persistedTheme = loadPersistedState().theme
applyDocumentTheme(previewTheme ?? resolveTheme(persistedTheme, systemPrefersDark()))

createApp(App).use(router).mount('#app')
