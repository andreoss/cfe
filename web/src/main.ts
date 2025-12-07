import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import { useDisplayStore } from './stores/display'

const app = createApp(App)

app.use(createPinia())
app.use(router)

useDisplayStore().apply()

app.mount('#app')
