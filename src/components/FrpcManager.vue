<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, NButton, NCard, NLog, NSpace,NSwitch } from 'naive-ui'
import type { LogInst } from 'naive-ui'
import hljs from 'highlight.js'
import ansiToHtml from 'ansi-to-html'
import { useLinkTunnelsStore } from '../stores/linkTunnels'

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
const logInst = ref<LogInst | null>(null) // 引用 n-log 实例
const autoScroll = ref(true) // 添加自动滚动控制
const logs = ref(localStorage.getItem('frpcLogs') || '')
const cleanupFunctions = ref<(() => void)[]>([])
const activeListeners = ref(new Map<string, boolean>())

// 添加日志滚动处理函数
const handleScroll = ({ scrollTop, scrollHeight, containerHeight }: any) => {
  // 当用户向上滚动时，禁用自动滚动
  if (scrollHeight - (scrollTop + containerHeight) > 50) {
    autoScroll.value = false
  } else {
    autoScroll.value = true
  }
}
// 统一的日志追加函数
const appendToLog = (content: string) => {
  logs.value += content
  localStorage.setItem('frpcLogs', logs.value)
  
  if (autoScroll.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom' })
    })
  }
}

// 修改现有的日志函数
const appendSystemLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  appendToLog(`[系统] [${timestamp}] ${message}\n`)
}

const appendTunnelLog = (tunnelId: string, message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'))
  appendToLog(`[${timestamp}] [<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}\n`)
}
// const appendLog = (message: string) => {
//   const timestamp = new Date().toLocaleTimeString()
//   appendToLog(`[系统] [${timestamp}] ${message}\n`)
// }

// 修改加载日志函数
const loadLogs = () => {
  logs.value = localStorage.getItem('frpcLogs') || ''
  if (autoScroll.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom', silent: true })
    })
  }
}

// 删除这里重复的 appendTunnelLog 函数声明
// 添加普通日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  logs.value += `[系统] [${timestamp}] ${message}\n`
  localStorage.setItem('frpcLogs', logs.value)
  
  if (autoScroll.value && logInst.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom' })
    })
  }
}

onMounted(async () => {
  loadLogs()
  
  // 删除这个重复的滚动代码
  // if (logInst.value) {
  //   logInst.value.scrollTo({ position: 'bottom' })
  // }

  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message)
    })
    cleanupFunctions.value.push(globalLogUnlisten)

    // 监听隧道事件
    const tunnelEventUnlisten = await listen('tunnel-event', (event: any) => {
      const { type, tunnelId, tunnelName } = event.payload
      // 当收到隧道启动事件时，立即设置日志监听器
      if (type === 'start' && !activeListeners.value.has(`frpc-log-${tunnelId}`)) {
        setupTunnelListener(tunnelId)
      }
      
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

    // 提取设置隧道监听器的逻辑为独立函数
    const setupTunnelListener = async (tunnelId: string) => {
      const listenerKey = `frpc-log-${tunnelId}`
      if (!activeListeners.value.has(listenerKey)) {
        const instanceLogUnlisten = await listen(listenerKey, (event: any) => {
          appendTunnelLog(tunnelId, event.payload.message)
        })
        activeListeners.value.set(listenerKey, true)
        cleanupFunctions.value.push(instanceLogUnlisten)
      }
    }

    // 为已保存的隧道设置监听器
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const id of Object.keys(states)) {
        await setupTunnelListener(id)
      }
    }

    // 监听外部隧道事件
    const linkTunnelsStore = useLinkTunnelsStore()
    watch(() => linkTunnelsStore.linkLaunchedTunnels, async (tunnels) => {
      for (const tunnelId of tunnels) {
        await setupTunnelListener(tunnelId)
      }
    }, { immediate: true })

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
    activeListeners.value.clear()
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
// logInst.value?.scrollTo({ position: 'bottom' })
// 删除这行重复的滚动代码
// logInst.value?.scrollTo({ position: 'bottom' })
</script>

<template>
  <n-space vertical>
    <n-card title="运行日志">
      <template #header-extra>
        <n-space>
          <n-switch v-model:value="autoScroll">
            <template #checked>自动滚动开启</template>
            <template #unchecked>自动滚动关闭</template>
          </n-switch>
          <n-button text type="primary" @click="clearLogs">
            清除日志
          </n-button>
        </n-space>
      </template>
      <n-log 
        :rows="25"
        :log="logs"
        :loading="false"
        :hljs="hljs"
        ref="logInst"
        @scroll="handleScroll"
        language="naive-log"
        trim
      />
    </n-card>
  </n-space>
</template>
