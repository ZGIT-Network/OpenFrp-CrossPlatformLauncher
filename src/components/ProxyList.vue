<script setup lang="ts">
import { ref, onMounted, onUnmounted,h, computed } from 'vue'
import { NCard, NSpace, NSwitch, NButton, NTooltip, useMessage, useNotification, NGrid, NGridItem, NText, NTag, NSkeleton, NScrollbar } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import axios from 'axios'
import { useLinkTunnelsStore } from '@/stores/linkTunnels'
import { listen } from '@tauri-apps/api/event'

const message = useMessage()
const notification = useNotification()
const tunnels = ref<any[]>([])
const loading = ref(false)
const loadingTunnels = ref<Set<string>>(new Set())

const linkTunnelsStore = useLinkTunnelsStore()



// 存储外部隧道信息
interface ExternalTunnel {
  id: string;
  status: 'running' | 'stopped';
}

const externalTunnels = ref<Map<string, ExternalTunnel>>(new Map())

// 从localStorage获取用户Token
const getUserToken = () => {
  return localStorage.getItem('userToken') || ''
}

// 获取隧道列表并检查状态
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
      tunnels.value = response.data.data.flatMap((group: any) =>
        group.proxies.map((proxy: any) => ({
          ...proxy,
          node: group.node,
          status: 'stopped' // 默认状态为停止
        }))
      )
      tunnels.value.sort((a, b) => a.id - b.id)

      // 加载保存的状态
      loadTunnelStates()

      // 立即检查所有隧道的实际运行状态
      await Promise.all(tunnels.value.map(tunnel => checkTunnelStatus(tunnel)))
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
    /启动.*成功/,               // 匹配 "xxx启动xxx成功xxx"
  ]
  // console.log(isSuccessLog)

  return successPatterns.some(pattern => pattern.test(log))
}

// 检查隧道状态
const checkTunnelStatus = async (tunnel: any) => {
  try {
    const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })
    tunnel.status = isRunning ? 'running' : 'stopped'
    if (!isRunning) {
      // 如果隧道已停止，确保移除标签
      linkTunnelsStore.removeLinkTunnel(tunnel.id.toString())
    }
  } catch (e) {
    console.error('检查隧道状态失败:', e)
  }
}

// 定期检查所有隧道状态
const checkAllTunnelsStatus = () => {
  tunnels.value.forEach(tunnel => checkTunnelStatus(tunnel))
}

// 添加复制函数
const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text)
    message.success('已复制到剪贴板')
  } catch (err) {
    message.error('复制失败')
    console.error('复制失败:', err)
  }
}

const toggleTunnel = async (tunnel: any) => {
  const token = getUserToken()
  try {
    loadingTunnels.value.add(tunnel.id.toString())

    const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })

    if (isRunning) {
      // 如果实际在运行，则停止
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'stop',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })

      await invoke('stop_frpc_instance', { id: tunnel.id.toString() })
      tunnel.status = 'stopped'
      // 当隧道停止时，移除"通过快速启动"标签
      linkTunnelsStore.removeLinkTunnel(tunnel.id.toString())
      message.info(`隧道 #${tunnel.id} ${tunnel.name} 已停止运行`)
      
      // 立即检查状态
      await checkTunnelStatus(tunnel)
    } else {
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'start',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })

      // 等待日志响应
      message.loading('正在启动隧道', {duration: 1000})
      let logMessage = ''
      console.log(logMessage)
      
      const logResult = await new Promise<{success: boolean, message: string}>((resolve) => {
        const timeout = setTimeout(() => resolve({success: false, message: '启动超时，请检查日志。'}), 5000)
        
        const logListener = listen(`frpc-log-${tunnel.id}`, (event: any) => {
          const log = event.payload.message
          logMessage = log
          
          if (isSuccessLog(log)) {
            clearTimeout(timeout)
            logListener.then(unlisten => unlisten())
            resolve({success: true, message: log})
          } else if (log.includes('启动失败')) {
            clearTimeout(timeout)
            logListener.then(unlisten => unlisten())
            // 提取错误信息
            const errorMatch = log.match(/启动失败:\s*(.+)/)
            const errorMessage = errorMatch ? errorMatch[1] : log
            resolve({success: false, message: errorMessage})
          }
        })
        
        invoke('start_frpc_instance', {
          id: tunnel.id.toString(),
          token: token,
          tunnelId: tunnel.id.toString()
        }).catch((error) => {
          clearTimeout(timeout)
          resolve({success: false, message: String(error)})
        })
      })

      if (logResult.success) {
        tunnel.status = 'running'
        notification.success({
          title: `隧道 ${tunnel.id}  ${tunnel.name} 启动成功`,
          description: `连接地址: ${tunnel.remote}`,
          content: () => h('div', [            
            h('span', `隧道 [ ${tunnel.name}  ] 启动成功, 请使用 [ ${tunnel.remote} ] 来连接服务\n`),
            h(NButton, {
              type: 'success',
              text: true,
              onClick: () => copyToClipboard(tunnel.remote)
            }, '复制连接地址')
          ]),
          duration: 5000
        })
        await sendNotification({ title: `隧道 #${tunnel.id}  ${tunnel.name} 启动成功`, body: `使用 ${tunnel.remote} 连接到服务` })
      } else {
        tunnel.status = 'stopped'
        message.error(`隧道启动失败: ${logResult.message}`)
        await sendNotification({ title: `隧道 #${tunnel.id}  ${tunnel.name} 启动失败`, body: logResult.message })
      }
    }
    saveTunnelStates()
  } catch (e) {
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'error',
        tunnelId: tunnel.id.toString(),
        tunnelName: tunnel.name
      }
    })

    message.error(`操作失败: ${e}`)
    // 操作失败后立即检查实际状态
    await checkTunnelStatus(tunnel)
  } finally {
    loadingTunnels.value.delete(tunnel.id.toString())
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

// 在恢复隧道状态前添加延迟
const restoreTunnelStates = async () => {
  try {
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const tunnel of tunnels.value) {
        if (states[tunnel.id] === 'running') {
          // 添加延迟确保 frpc 完全初始化
          await new Promise(resolve => setTimeout(resolve, 1000))
          await toggleTunnel(tunnel)
        }
      }
    }
  } catch (e) {
    console.error('恢复隧道状态失败:', e)
  }
}

// 获取只在快速启动列表显示的隧道
const getLinkOnlyTunnels = computed(() => {
  const result = new Set(linkTunnelsStore.linkLaunchedTunnels)
  tunnels.value.forEach(tunnel => {
    if (result.has(tunnel.id.toString())) {
      result.delete(tunnel.id.toString())
    }
  })
  
  // 将外部隧道添加到 Map 中
  result.forEach(tunnelId => {
    if (!externalTunnels.value.has(tunnelId)) {
      externalTunnels.value.set(tunnelId, {
        id: tunnelId,
        status: 'running'
      })
    }
  })
  
  return result
})

// 停止外部隧道
const stopExternalTunnel = async (tunnelId: string) => {
  try {
    // 发送停止事件到日志系统
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'stop',
        tunnelId: tunnelId,
        tunnelName: `[快速启动]`
      }
    })

    await invoke('stop_frpc_instance', { id: tunnelId })
    linkTunnelsStore.removeLinkTunnel(tunnelId)
    externalTunnels.value.delete(tunnelId)
    message.success('停止外部隧道成功')
  } catch (e) {
    // 发送错误事件到日志系统
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'error',
        tunnelId: tunnelId,
        tunnelName: `[快速启动]`
      }
    })
    message.error(`停止外部隧道失败: ${e}`)
  }
}

// 检查外部隧道状态
const checkExternalTunnelStatus = async (tunnelId: string) => {
  try {
    const isRunning = await invoke('check_frpc_status', { id: tunnelId })
    if (!isRunning) {
      linkTunnelsStore.removeLinkTunnel(tunnelId)
      externalTunnels.value.delete(tunnelId)
    }
  } catch (e) {
    console.error('检查外部隧道状态失败:', e)
  }
}

onMounted(async () => {
  await requestNotificationPermission()
  await fetchProxyList()

  // 检查是否需要自动恢复隧道
  const shouldRestore = localStorage.getItem('autoRestoreTunnels') === 'true'
  const isAutoStart = window.location.search.includes('autostart=true')
  
  if (shouldRestore && isAutoStart) {
    await restoreTunnelStates()
  }

  // 设置定期检查隧道状态的定时器
  const statusCheckInterval = setInterval(checkAllTunnelsStatus, 5000)
  const listRefreshInterval = setInterval(fetchProxyList, 45000)

  // 检查外部隧道状态的定时器
  const checkInterval = setInterval(() => {
    externalTunnels.value.forEach((_, tunnelId) => {
      checkExternalTunnelStatus(tunnelId)
    })
  }, 5000)

  // 组件卸载时清理定时器
  onUnmounted(() => {
    clearInterval(statusCheckInterval)
    clearInterval(listRefreshInterval)
    clearInterval(checkInterval)
  })
})
</script>

<template>
  <n-scrollbar>
    <n-space vertical>
      <!-- 外部隧道卡片 -->
      <n-skeleton v-if="loading" :height="3" />
      <n-card v-else-if="getLinkOnlyTunnels.size > 0" title="外部快速启动隧道">
        <n-space vertical>
          <n-space v-for="tunnelId in getLinkOnlyTunnels" :key="tunnelId" justify="space-between">
            <n-space>
              <n-text>外部隧道 #{{ tunnelId }}</n-text>
              <n-tag type="warning" size="small">不属于当前用户</n-tag>
              <n-tag   type="info" 
                       size="small">
                  快速启动
                </n-tag>
            </n-space>
            <n-button 
              type="error" 
              size="small" 
              @click="stopExternalTunnel(tunnelId)"
            >
              停止
            </n-button>
          </n-space>
        </n-space>
      </n-card>

      <!-- 原有的隧道列表 -->
      <n-card>
        <n-space>
          <n-button @click="fetchProxyList" :loading="loading">
            刷新隧道列表
          </n-button>
        </n-space>
      </n-card>

      <n-skeleton v-if="loading" :height="3" />
      <n-grid v-else :cols="2" :x-gap="12" :y-gap="12">
        <n-grid-item v-for="tunnel in tunnels" :key="tunnel.id" style="display: flex;">
          <n-card :title="'隧道 #' + tunnel.id + ' ' + tunnel.name" :bordered="false" size="small"
            style="flex: 1; height: 100%;">
            <template #header-extra>
              <n-space>
                <n-tag v-if="linkTunnelsStore.linkLaunchedTunnels.has(tunnel.id.toString())" 
                       type="info" 
                       size="small">
                  通过快速启动
                </n-tag>
                <n-tag :type="getTypeColor(tunnel.type)">
                  {{ tunnel.type.toUpperCase() }}
                </n-tag>
              </n-space>
            </template>
            <n-space vertical size="small">
              <n-space>
                <n-text depth="3">节点</n-text>
                <n-text>{{ tunnel.node }}</n-text>
              </n-space>

              <n-space>
                <n-text depth="3">本地地址</n-text>
                <n-text>{{ tunnel.local }}</n-text>
              </n-space>
              <n-space>
                <n-text depth="3">远程地址</n-text>
                <n-text>{{ tunnel.remote }}</n-text>
              </n-space>

              <n-space justify="end">
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-switch :value="tunnel.status === 'running'" @update:value="() => toggleTunnel(tunnel)"
                      :loading="loadingTunnels.has(tunnel.id.toString())"
                      :disabled="loadingTunnels.has(tunnel.id.toString())" />
                  </template>
                  控制隧道启动/停止
                </n-tooltip>
              </n-space>
            </n-space>
          </n-card>
        </n-grid-item>
      </n-grid>
    </n-space>
  </n-scrollbar>
</template>
<style scoped>
.n-card {
  transition: all 0.3s;
}

.n-card:hover {
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
}
</style>
