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
app.config.errorHandler = (err, instance, info) => {
  console.error('全局错误:', err)
  console.error('错误组件:', instance)
  console.error('错误信息:', info)
}

app.config.warnHandler = (msg, instance, trace) => {
  console.warn('警告:', msg)
  console.warn('警告组件:', instance)
  console.warn('调用栈:', trace)
}
app.mount('#app')
