<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, useDialog, NButton, NCard, NLog, NSpace } from 'naive-ui'

const message = useMessage()
const dialog = useDialog()
const logs = ref('')

// 存储所有监听器的清理函数
const cleanupFunctions = ref<(() => void)[]>([])

// 从 localStorage 加载日志
const loadLogs = () => {
  const savedLogs = localStorage.getItem('frpcLogs')
  if (savedLogs) {
    logs.value = savedLogs
  }
}

// 保存日志到 localStorage
watch(logs, (newValue) => {
  localStorage.setItem('frpcLogs', newValue)
})

onMounted(async () => {
  // 加载已保存的日志
  loadLogs()

  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      logs.value += event.payload.message + '\n'
    })
    cleanupFunctions.value.push(globalLogUnlisten)

    // 监听各个实例的日志
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const id of Object.keys(states)) {
        if (states[id] === 'running') {
          const instanceLogUnlisten = await listen(`frpc-log-${id}`, (event: any) => {
            logs.value += `[隧道 ${id}] ${event.payload.message}\n`
          })
          cleanupFunctions.value.push(instanceLogUnlisten)
        }
      }
    }

    // 监听下载提示
    const downloadUnlisten = await listen('need_download', async () => {
      dialog.warning({
        title: '提示',
        content: '未检测到 frpc，是否立即下载？',
        positiveText: '下载',
        negativeText: '取消',
        onPositiveClick: () => {
          // 处理下载逻辑
        }
      })
    })
    cleanupFunctions.value.push(downloadUnlisten)

  } catch (error) {
    console.error('设置日志监听器时出错:', error)
    message.error('设置日志监听器失败')
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
    logs.value = newValue.slice(-MAX_LOG_LENGTH / 2)
  }
})
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
        trim
      />
    </n-card>
  </n-space>
</template>

<style scoped>
.n-card {
  margin-bottom: 16px;
}
</style>