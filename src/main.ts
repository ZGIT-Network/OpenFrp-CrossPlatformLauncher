import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'

const app = createApp(App)
app.use(router)
const pinia = createPinia()
app.use(pinia)
pinia.use(piniaPluginPersistedstate)
app.directive('external', {
  mounted(el: any) {
    const links = el.getElementsByTagName('a')
    for (const link of links) {
      if (link.host !== window.location.host) {
        link.setAttribute('target', '_blank')
        link.setAttribute('rel', 'noopener noreferrer')
      }
    }
  }
})
app.mount('#app')
