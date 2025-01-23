<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NCard, NSpace, NSwitch, NButton, NTooltip, useMessage, useNotification, NGrid, NGridItem, NText, NTag, NSkeleton } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import axios from 'axios'

const message = useMessage()
const notification = useNotification()
const tunnels = ref<any[]>([])
const loading = ref(false)

// 从localStorage获取用户Token
const getUserToken = () => {
  return localStorage.getItem('userToken') || ''
}

// 获取隧道列表
const fetchProxyList = async () => {
  const token = getUserToken()
  if (!token) {
    message.warning('请先在设置页面配置用户密钥')
    return
  }

  loading.value = true
  try {
    const response = await axios.get(`https://api.openfrp.net/api`, {
      params: {
        action: 'getallproxies',
        user: token
      }
    })

    if (response.data.success) {
      // 将数据扁平化处理并按id排序
      tunnels.value = response.data.data.flatMap((group: any) => 
        group.proxies.map((proxy: any) => ({
          ...proxy,
          node: group.node,
          status: 'stopped' // 默认状态
        }))
      )
      // 按id排序
      tunnels.value.sort((a, b) => a.id - b.id)
      // 加载保存的状态
      loadTunnelStates()
    } else {
      message.error('获取隧道列表失败')
    }
  } catch (e) {
    message.error(`请求失败: ${e}`)
  } finally {
    loading.value = false
  }
}

// 请求通知权限
const requestNotificationPermission = async () => {
  try {
    let permissionGranted = await isPermissionGranted()
    if (!permissionGranted) {
      const permission = await requestPermission()
      permissionGranted = permission === 'granted'
    }
    return permissionGranted
  } catch (e) {
    console.error('获取通知权限失败:', e)
    return false
  }
}

// 检查日志是否包含成功信息的函数
const isSuccessLog = (log: string): boolean => {
  const successPatterns = [
    /start.*success/i,          // 匹配 "start xxx success"
    /启动成功/,               // 匹配 "xxx启动xxx成功xxx"
    /login.*success/i,          // 匹配 "login xxx success"
    /tunnel.*established/i,     // 匹配 "tunnel xxx established"
    /connection.*established/i,  // 匹配 "connection xxx established"
    /connected.*success/i       // 匹配 "connected xxx success"
  ]
  
  return successPatterns.some(pattern => pattern.test(log))
}

// 检查隧道状态
const checkTunnelStatus = async (tunnel: any) => {
  try {
    const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })
    if (isRunning !== (tunnel.status === 'running')) {
      tunnel.status = isRunning ? 'running' : 'stopped'
      saveTunnelStates()
    }
  } catch (e) {
    console.error('检查隧道状态失败:', e)
  }
}

// 定期检查所有隧道状态
const checkAllTunnelsStatus = () => {
  tunnels.value.forEach(tunnel => checkTunnelStatus(tunnel))
}

const toggleTunnel = async (tunnel: any) => {
  const token = getUserToken()
  try {
    if (tunnel.status === 'running') {
      await invoke('stop_frpc_instance', { id: tunnel.id.toString() })
      tunnel.status = 'stopped'
    } else {
      // 检查是否已在运行
      const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })
      if (isRunning) {
        message.warning('该隧道已在运行中')
        tunnel.status = 'running'
        return
      }

      let lastLog = ''
      const unlistenLog = await listen(`frpc-log-${tunnel.id}`, (event: any) => {
        const logMessage = event.payload.message
        console.log('收到日志:', logMessage) // 调试用
        lastLog = logMessage
        console.log(unlistenLog)
        
        if (isSuccessLog(logMessage)) {
          message.success('隧道启动成功')
          notification.success({
            title: '隧道启动成功',
            content: `隧道 ${tunnel.name} (ID: ${tunnel.id}) 已成功启动\n${lastLog}`,
            duration: 5000
          })
          
          requestNotificationPermission().then(granted => {
            if (granted) {
              sendNotification({
                title: '隧道启动成功',
                body: `隧道 ${tunnel.name} (ID: ${tunnel.id}) 已成功启动\n${lastLog}`
              })
            }
          })
        }
      })

      await invoke('start_frpc_instance', {
        id: tunnel.id.toString(),
        token: token,
        tunnelId: tunnel.id.toString()
      })
      tunnel.status = 'running'
    }
    saveTunnelStates()
  } catch (e) {
    message.error(`操作失败: ${e}`)
    await checkTunnelStatus(tunnel)
  }
}

// 保存隧道状态到localStorage
const saveTunnelStates = () => {
  const states = tunnels.value.reduce((acc, tunnel) => {
    acc[tunnel.id.toString()] = tunnel.status
    return acc
  }, {} as Record<string, string>)
  localStorage.setItem('tunnelStates', JSON.stringify(states))
}

// 从localStorage加载隧道状态
const loadTunnelStates = () => {
  const savedStates = localStorage.getItem('tunnelStates')
  if (savedStates) {
    const states = JSON.parse(savedStates)
    tunnels.value.forEach(tunnel => {
      if (states[tunnel.id.toString()]) {
        tunnel.status = states[tunnel.id.toString()]
      }
    })
  }
}

const getTypeColor = (type: string) => {
  const colors: Record<string, "error" | "default" | "primary" | "info" | "success" | "warning"> = {
    tcp: 'success',
    udp: 'warning',
    http: 'info',
    https: 'primary'
  }
  return colors[type.toLowerCase()] || 'default'
}

onMounted(async () => {
  await requestNotificationPermission()
  await fetchProxyList()
  // 定期检查隧道状态
  setInterval(checkAllTunnelsStatus, 5000)
  // 定期刷新隧道列表
  setInterval(fetchProxyList, 30000)
})
</script>

<template>
  <n-space vertical>
    <n-card>
      <n-space>
        <n-button @click="fetchProxyList" :loading="loading">
          刷新隧道列表
        </n-button>
      </n-space>
    </n-card>
    <n-skeleton v-if="loading" height="3"></n-skeleton>
    <n-grid v-else :cols="2" :x-gap="12" :y-gap="12">
      <n-grid-item v-for="tunnel in tunnels" :key="tunnel.id" style="display: flex;">
        <n-card :title="tunnel.name" :bordered="false" size="small" style="flex: 1; height: 100%;">
          <template #header-extra>
            <n-tag :type="getTypeColor(tunnel.type)">
              {{ tunnel.type.toUpperCase() }}
            </n-tag>
          </template>
          <n-space vertical size="small">
            <n-space justify="space-between">
              <n-text depth="3">ID</n-text>
              <n-text># {{ tunnel.id }}</n-text>
            </n-space>
            <n-space justify="space-between">
              <n-text depth="3">远程地址</n-text>
              <n-text>{{ tunnel.remote }}</n-text>
            </n-space>
            <n-space justify="space-between">
              <n-text depth="3">本地地址</n-text>
              <n-text>{{ tunnel.local }}</n-text>
            </n-space>
            <n-space justify="space-between">
              <n-text depth="3">节点</n-text>
              <n-text>{{ tunnel.node }}</n-text>
            </n-space>
            <n-space justify="end">
              <n-tooltip trigger="hover">
                <template #trigger>
                  <n-switch
                    :value="tunnel.status === 'running'"
                    @update:value="() => toggleTunnel(tunnel)"
                  />
                </template>
                控制隧道启动/停止
              </n-tooltip>
            </n-space>
          </n-space>
        </n-card>
      </n-grid-item>
    </n-grid>
  </n-space>
</template>

<style scoped>
.n-card {
  transition: all 0.3s;
}
.n-card:hover {
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
}
</style>