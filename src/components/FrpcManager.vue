<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, NButton, NCard, NLog, NSpace } from 'naive-ui'
import type { LogInst } from 'naive-ui'
import hljs from 'highlight.js'
import ansiToHtml from 'ansi-to-html'

const convert = new ansiToHtml({
  fg: '#FFF',
  bg: '#000',
  newline: true,
  escapeXML: false,
  stream: false,
  colors: {
    '1': '#FB9FB1',  // 红色
    '2': '#ACC267',  // 绿色
    '3': '#DDB26F',  // 黄色
    '4': '#6FC2EF',  // 蓝色
    '5': '#E1A3EE',  // 紫色
    '6': '#12CFC0',  // 青色
    '7': '#D0D0D0',  // 白色
    '9': '#F6363F'   // 亮红色
  }
})

const message = useMessage()
const logs = ref('')
const logInst = ref<LogInst | null>(null) // 引用 n-log 实例

// 存储所有监听器的清理函数
const cleanupFunctions = ref<(() => void)[]>([])

// 从 localStorage 加载日志
const loadLogs = () => {
  logs.value = localStorage.getItem('frpcLogs') || ''
  logInst.value?.scrollTo({ position: 'bottom', silent: true },)
}

// 添加系统日志的辅助函数
const appendSystemLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  logs.value += `[系统] [${timestamp}] ${message}\n`
  localStorage.setItem('frpcLogs', logs.value)
}

// 修改隧道日志的辅助函数
const appendTunnelLog = (tunnelId: string, message: string) => {
  // 处理 ANSI 转义序列
  const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'))
  logs.value += `[<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}\n`
  localStorage.setItem('frpcLogs', logs.value)
}

// 添加普通日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  logs.value += `[系统] [${timestamp}] ${message}\n`
  localStorage.setItem('frpcLogs', logs.value)
}


onMounted(async () => {
  // 加载已保存的日志
  loadLogs()
  
  // 确保日志滚动到最底部
  if (logInst.value) {
    logInst.value.scrollTo({ position: 'bottom' })
    
  }

  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message)
    })
    cleanupFunctions.value.push(globalLogUnlisten)

    // 监听隧道事件
    const tunnelEventUnlisten = await listen('tunnel-event', (event: any) => {
      const { type, tunnelId, tunnelName } = event.payload
      switch (type) {
        case 'start':
          appendSystemLog(`<span style="color: #2080f0">开始启动隧道 #${tunnelId} ${tunnelName}</span>`)
          break
        case 'stop':
          appendSystemLog(`<span style="color: #2080f0">停止隧道 #${tunnelId} ${tunnelName}</span>  `)
          break
        case 'success':
          appendSystemLog(`<span style="color: #2080f0">隧道 #${tunnelId} ${tunnelName} 启动成功</span>`)
          break
        case 'error':
          appendSystemLog(`<span style="color: #2080f0">隧道 #${tunnelId} ${tunnelName} 发生错误</span>`)
          break
      }
    })
    cleanupFunctions.value.push(tunnelEventUnlisten)

    // 立即为所有可能的隧道ID设置监听器
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const id of Object.keys(states)) {
        const instanceLogUnlisten = await listen(`frpc-log-${id}`, (event: any) => {
          appendTunnelLog(id, event.payload.message)
        })
        cleanupFunctions.value.push(instanceLogUnlisten)
      }
    }
  } catch (error) {
    console.error('设置日志监听器时出错:', error)
    appendSystemLog(`设置日志监听器失败: ${error}`)
    message.error('设置日志监听器失败')
  }

  // 每秒更新一次日志显示
  const updateInterval = setInterval(loadLogs, 1000)

  // 监听 logs 的变化并滚动到底部
  watch(logs, () => {
    if (logInst.value) {
      logInst.value.scrollTo({ position: 'bottom' })   
     }
  })

  onUnmounted(() => {
    // 清理所有监听器
    cleanupFunctions.value.forEach(cleanup => {
      try {
        cleanup()
      } catch (error) {
        console.error('清理监听器时出错:', error)
      }
    })
    cleanupFunctions.value = []
    clearInterval(updateInterval)
  })
})


// 清除日志
const clearLogs = () => {
  logs.value = ''
  localStorage.removeItem('frpcLogs')
}

// 防止日志过长，定期清理
const MAX_LOG_LENGTH = 50000 // 最大日志长度
watch(logs, (newValue) => {
  if (newValue.length > MAX_LOG_LENGTH) {
    // 保留后半部分的日志
    const truncatedLogs = newValue.slice(-MAX_LOG_LENGTH / 2)
    logs.value = truncatedLogs
    localStorage.setItem('frpcLogs', truncatedLogs)
  }
})
logInst.value?.scrollTo({ position: 'bottom' })

</script>

<template>
  <n-space vertical>
    <n-card title="运行日志">
      <template #header-extra>
        <n-button text type="primary" @click="clearLogs">
          清除日志
        </n-button>
      </template>
      <n-log 
        :rows="20"
        :log="logs"
        :loading="false"
        :hljs="hljs"
        ref="logInst"
        language="naive-log"
        trim
      />
    </n-card>
  </n-space>
</template>
