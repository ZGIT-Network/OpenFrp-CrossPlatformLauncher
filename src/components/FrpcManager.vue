<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, NButton, NCard, NLog, NSpace, NSwitch, NSelect } from 'naive-ui'
import type { LogInst, SelectOption } from 'naive-ui'
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
const rawLogs = ref(localStorage.getItem('frpcLogs') || '')
const logs = ref('')
const cleanupFunctions = ref<(() => void)[]>([])
const activeListeners = ref(new Map<string, boolean>())

// 添加隧道筛选功能
const tunnelOptions = ref<SelectOption[]>([
  { label: '全部日志', value: 'all' }
])
const selectedTunnel = ref('all')

// 解析日志并按隧道ID分类
const parsedLogs = computed(() => {
  const logLines = rawLogs.value.split('\n')
  const result: Record<string, string[]> = { all: [] }
  
  logLines.forEach((line: any) => {
    if (line.trim()) {
      result.all.push(line)
      
      // 检查是否包含隧道ID - 修改正则表达式以正确匹配所有可能的格式
      const tunnelMatch = line.match(/\[隧道\s+(\d+)\]/) || 
                          line.match(/隧道\s+#(\d+)/) || 
                          line.match(/\[<span style="color: #2080f0">隧道\s+(\d+)<\/span>\]/)
      
      if (tunnelMatch && tunnelMatch[1]) {
        const tunnelId = tunnelMatch[1]
        if (!result[tunnelId]) {
          result[tunnelId] = []
        }
        result[tunnelId].push(line)
      } else if (line.includes('[系统]')) {
        // 系统日志添加到所有隧道的日志中
        Object.keys(result).forEach(key => {
          if (key !== 'all') {
            result[key].push(line)
          }
        })
      }
    }
  })
  
  // 调试输出
  console.log('解析的日志分类:', Object.keys(result));
  
  return result
})

// 根据选择的隧道过滤日志
watch([parsedLogs, selectedTunnel], () => {
  if (selectedTunnel.value === 'all') {
    logs.value = parsedLogs.value.all.join('\n')
  } else {
    // 如果选定隧道的日志不存在，显示空字符串
    logs.value = (parsedLogs.value[selectedTunnel.value] || []).join('\n')
  }
  
  // 添加调试日志
  console.log(`已选择隧道: ${selectedTunnel.value}, 日志行数: ${logs.value.split('\n').length}`);
  
  if (autoScroll.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom', silent: true })
    })
  }
}, { immediate: true })

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
  rawLogs.value += content
  localStorage.setItem('frpcLogs', rawLogs.value)
  
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

// 修复类型错误
const appendTunnelLog = (tunnelId: string, message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'))
  // 新增：检查是否是启动成功消息，如果是，发送一个特殊的成功事件
  if (message.includes('start proxy success') || 
      message.includes('start tunnel success') ||
      message.includes('隧道启动成功')) {
    // 发送一个特殊的成功事件，ProxyList组件可以监听这个事件
    window.dispatchEvent(new CustomEvent(`tunnel-${tunnelId}-success`, {
      detail: { message: message }
    }))
  }
  
  appendToLog(`[${timestamp}] [<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}\n`)
  
  // 如果是新隧道，添加到选项中
  if (!tunnelOptions.value.some((option: SelectOption) => option.value === tunnelId)) {
    tunnelOptions.value.push({
      label: `隧道 ${tunnelId}`,
      value: tunnelId
    })
  }
}

// 添加普通日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  appendToLog(`[系统] [${timestamp}] ${message}\n`)
}

// 修改加载日志函数
const loadLogs = () => {
  rawLogs.value = localStorage.getItem('frpcLogs') || ''
}

// 修改设置隧道监听器的逻辑
const setupTunnelListener = async (tunnelId: string) => {
  const listenerKey = `frpc-log-${tunnelId}`
  
  // 先清理可能存在的旧监听器
  if (activeListeners.value.has(listenerKey)) {
    console.log(`重新设置隧道 ${tunnelId} 的日志监听器`)
    // 找到并清理旧的监听器
    const index = cleanupFunctions.value.findIndex(
      fn => fn.toString().includes(listenerKey)
    );
    if (index !== -1) {
      try {
        await cleanupFunctions.value[index]();
        cleanupFunctions.value.splice(index, 1);
      } catch (e) {
        console.error(`清理隧道 ${tunnelId} 旧监听器失败:`, e);
      }
    }
    activeListeners.value.delete(listenerKey);
  }
  
  // 设置新的监听器
  console.log(`设置隧道 ${tunnelId} 的日志监听器`);
  try {
    const instanceLogUnlisten = await listen(listenerKey, (event: any) => {
      console.log(`收到隧道 ${tunnelId} 的日志:`, event.payload.message);
      appendTunnelLog(tunnelId, event.payload.message);
    });
    activeListeners.value.set(listenerKey, true);
    cleanupFunctions.value.push(instanceLogUnlisten);
  } catch (e) {
    console.error(`设置隧道 ${tunnelId} 监听器失败:`, e);
  }
}

onMounted(async () => {
  loadLogs()
  
  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message)
    })
    cleanupFunctions.value.push(globalLogUnlisten)

    // 监听隧道事件，修改为每次都重新设置监听器
    const tunnelEventUnlisten = await listen('tunnel-event', async (event: any) => {
      const { type, tunnelId, tunnelName } = event.payload
      
      // 对于启动相关事件，始终重新设置监听器
      if (type === 'start' || type === 'prepare') {
        await setupTunnelListener(tunnelId)
      }
      
      // 如果是新隧道，添加到选项中
      if (!tunnelOptions.value.some((option: any) => option.value === tunnelId)) {
        tunnelOptions.value.push({
          label: `隧道 ${tunnelId} (${tunnelName})`,
          value: tunnelId
        })
      }
      
      switch (type) {
        case 'prepare':
          appendSystemLog(`<span style="color: #2080f0">准备启动隧道 #${tunnelId} ${tunnelName}</span>`)
          break
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

    // 为已保存的隧道设置监听器
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const id of Object.keys(states)) {
        await setupTunnelListener(id)
        
        // 添加到选项中
        if (!tunnelOptions.value.some(option => option.value === id)) {
          tunnelOptions.value.push({
            label: `隧道 ${id}`,
            value: id
          })
        }
      }
    }

    // 监听外部隧道事件
    const linkTunnelsStore = useLinkTunnelsStore()
    watch(() => linkTunnelsStore.linkLaunchedTunnels, async (tunnels) => {
      for (const tunnelId of tunnels) {
        await setupTunnelListener(tunnelId)
        
        // 添加到选项中
        if (!tunnelOptions.value.some(option => option.value === tunnelId)) {
          tunnelOptions.value.push({
            label: `隧道 ${tunnelId}`,
            value: tunnelId
          })
        }
      }
    }, { immediate: true })

  } catch (error) {
    console.error('设置日志监听器时出错:', error)
    appendSystemLog(`设置日志监听器失败: ${error}`)
    message.error('设置日志监听器失败')
  }

  // 每秒更新一次日志显示
  const updateInterval = setInterval(loadLogs, 1000)

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
  rawLogs.value = ''
  localStorage.removeItem('frpcLogs')
}

// 防止日志过长，定期清理
const MAX_LOG_LENGTH = 50000 // 最大日志长度
watch(rawLogs, (newValue) => {
  if (newValue.length > MAX_LOG_LENGTH) {
    // 保留后半部分的日志
    const truncatedLogs = newValue.slice(-MAX_LOG_LENGTH / 2)
    rawLogs.value = truncatedLogs
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
          <n-select 
            v-model:value="selectedTunnel" 
            :options="tunnelOptions" 
            placeholder="选择隧道" 
            style="width: 180px"
          />
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
