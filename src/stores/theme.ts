import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  // 从localStorage获取初始主题，默认为暗色
  const isDark = ref(localStorage.getItem('theme') === 'dark' || 
                     localStorage.getItem('theme') === null)
  
  // 设置主题
  function setTheme(dark: boolean) {
    isDark.value = dark
    // 保存主题设置到localStorage
    localStorage.setItem('theme', dark ? 'dark' : 'light')
    
    // 将主题应用到文档
    if (dark) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
    
    // 通知Tauri后端主题变化（如果需要）
    try {
      // @ts-ignore
      if (window.__TAURI__) {
        // @ts-ignore
        window.__TAURI__.invoke('set_theme', { dark })
      }
    } catch (e) {
      console.error('无法设置系统主题:', e)
    }
  }
  
  return { isDark, setTheme }
}) 