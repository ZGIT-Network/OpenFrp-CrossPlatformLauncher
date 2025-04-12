import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import { externalDirective } from './utils/linkHandler'

const app = createApp(App)
app.use(router)
const pinia = createPinia()
app.use(pinia)
pinia.use(piniaPluginPersistedstate)
app.directive('external', externalDirective)
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

// 挂载应用
app.mount('#app');
