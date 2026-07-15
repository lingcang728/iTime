import { createApp } from 'vue'
import App from './App.vue'
import { router } from './router'
import './styles/tokens.css'
import './styles/global.css'

const query = new URLSearchParams(window.location.search)
if (query.get('reference') === 'compact') document.documentElement.dataset.reference = 'compact'
if (query.get('theme') === 'dark') document.documentElement.dataset.theme = 'dark'

createApp(App).use(router).mount('#app')
