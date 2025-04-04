<script setup lang="ts">
import { ref, onMounted, onUnmounted,h, computed, nextTick, watch,inject,Ref } from 'vue'
import { NCard, NSpace, NSwitch, NButton, NTooltip, useMessage, useNotification, NGrid, NGridItem, NText, NTag, NSkeleton, NScrollbar } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import axios from 'axios'
import { useLinkTunnelsStore } from '@/stores/linkTunnels'
import { listen } from '@tauri-apps/api/event'
import { useRoute } from 'vue-router'

import frpApiGetUserProxies from '@/requests/frpApi/frpApiGetUserProxies';


const message = useMessage()
const notification = useNotification()
const tunnels = ref<any[]>([])
const loading = ref(false)
const loadingTunnels = ref<Set<string>>(new Set())


  
const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');
const userInfo = userInfoObj?.userInfo;


  

const linkTunnelsStore = useLinkTunnelsStore()

const route = useRoute()

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
  // 修改这里：确保从用户信息中获取 token
  const token = userInfo?.value?.token

  if (!token) {
    loading.value = false
    tunnels.value = [] // 清空隧道列表
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
      tunnels.value = [] // 清空隧道列表
    }
  } catch (e) {
    tunnels.value = [] // 清空隧道列表
    console.error('获取隧道列表失败:', e)
  } finally {
    loading.value = false
  }
}
// 添加对 userInfo 的监听
watch(() => userInfo?.value?.token, (newToken) => {
  if (!newToken) {
    // 当 token 不存在时（退出登录），清空隧道列表
    tunnels.value = []
    loading.value = false
  }
}, { immediate: true })

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
    /隧道.*成功/,               // 匹配 "隧道xxx成功xxx"
    /success/i,                 // 匹配任何包含success的消息
    /connected/i,               // 匹配连接成功的消息
    /running/i                  // 匹配运行中的消息
  ]

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
  // 修改这里：优先使用 userInfo 中的 token
  const token = userInfo?.value?.token

  if (!token) {
    message.error('未获取到用户 token，请确保已登录')
    return
  }

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
      saveTunnelStates()
    } else {
      // 确保为每次启动重新设置日志监听器
      console.log(`隧道 ${tunnel.id} 准备执行启动命令...`)
      
      // 先触发准备事件
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'prepare',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })
      
      // 等待足够的时间确保监听器设置好
      await new Promise(resolve => setTimeout(resolve, 1500))
      
      console.log(`监听器设置完成，开始启动隧道 ${tunnel.id}...`)
      
      // 然后启动隧道
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
      
      const logResult = await new Promise<{success: boolean, message: string}>((resolve) => {
        console.log(`等待隧道 ${tunnel.id} 的日志响应...`)
        
        // 增加超时时间到15秒
        const timeout = setTimeout(() => {
          console.log(`隧道 ${tunnel.id} 启动超时，检查隧道状态...`)
          // 超时后检查隧道实际状态
          invoke('check_frpc_status', { id: tunnel.id.toString() })
            .then(isRunning => {
              if (isRunning) {
                resolve({success: true, message: '隧道已经启动成功，但未收到成功日志。'})
              } else {
                resolve({success: false, message: '启动超时，请检查日志。'})
              }
            })
            .catch(() => resolve({success: false, message: '启动超时，检查状态失败。'}))
        }, 15000)
        
        // 监听成功事件
        const successEventListener = (event: any) => {
          console.log(`接收到隧道 ${tunnel.id} 的成功事件:`, event)
          clearTimeout(timeout)
          window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
          resolve({success: true, message: event.detail.message})
        }
        window.addEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
        
        // 监听日志事件
        console.log(`开始监听隧道 ${tunnel.id} 的日志...`)
        
        const logListener = listen(`frpc-log-${tunnel.id}`, (event: any) => {
          const log = event.payload.message
          logMessage = log
          
          // 改进成功检测逻辑
          if (isSuccessLog(log) || 
              log.includes('start proxy success') || 
              log.includes('start tunnel success') ||
              log.includes('隧道启动成功')) {
            clearTimeout(timeout)
            logListener.then(unlisten => unlisten())
            window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
            resolve({success: true, message: log})
          } else if (log.includes('启动失败') || log.includes('failed') || log.includes('error')) {
            clearTimeout(timeout)
            logListener.then(unlisten => unlisten())
            window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
            // 提取错误信息
            const errorMatch = log.match(/启动失败:\s*(.+)/)
            const errorMessage = errorMatch ? errorMatch[1] : log
            resolve({success: false, message: errorMessage})
          }
        })
        
        // 修复这里：添加 invoke 调用的调试日志
        console.log(`执行启动命令：隧道 ${tunnel.id}...`)
        invoke('start_frpc_instance', {
          id: tunnel.id.toString(),
          token: token,
          tunnelId: tunnel.id.toString()
        }).catch((error) => {
          console.error(`启动隧道 ${tunnel.id} 失败:`, error)
          clearTimeout(timeout)
          logListener.then(unlisten => unlisten())
          resolve({success: false, message: String(error)})
        })
      })

      if (logResult.success) {
        tunnel.status = 'running'
        saveTunnelStates() // 保存运行状态
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
        saveTunnelStates() // 保存失败状态
        message.error(`隧道启动失败: ${logResult.message}`)
        await sendNotification({ title: `隧道 #${tunnel.id}  ${tunnel.name} 启动失败`, body: logResult.message })
      }
    }
  } catch (e) {
    tunnel.status = 'stopped'
    saveTunnelStates() // 保存错误状态
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
// 保存隧道状态
const saveTunnelStates = () => {
  // 只保存当前正在运行的隧道
  const states: Record<string, string> = {}
  tunnels.value.forEach(tunnel => {
    // 检查隧道是否真的在运行
    if (tunnel.status === 'running') {
      states[tunnel.id] = tunnel.status
    }
  })
  console.log('Saving current running tunnel states:', states)
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

// 恢复隧道的函数
const restoreTunnels = async () => {
  const savedStates = localStorage.getItem('tunnelStates')
  console.log('Attempting to restore tunnels, saved states:', savedStates)
  
  if (savedStates) {
    const states = JSON.parse(savedStates)
    const tunnelsToRestore = tunnels.value.filter(tunnel => states[tunnel.id] === 'running')
    
    console.log('Found tunnels to restore:', tunnelsToRestore.map(t => t.id))
    
    // 先等待一段时间确保系统完全就绪
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    // 逐个启动隧道，确保每个都有足够的时间初始化
    for (const tunnel of tunnelsToRestore) {
      try {
        console.log('Starting tunnel:', tunnel.id)
        // 确保上一个隧道的状态已经稳定
        await new Promise(resolve => setTimeout(resolve, 2000))
        await toggleTunnel(tunnel)
        // 等待隧道启动完成
        await new Promise(resolve => setTimeout(resolve, 3000))
        console.log('Tunnel started:', tunnel.id)
      } catch (e) {
        console.error(`自动启动隧道 ${tunnel.id} 失败:`, e)
        continue
      }
    }
    
    // 最后再次保存所有状态
    saveTunnelStates()
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
  console.log('ProxyList mounted')
  await requestNotificationPermission()
  await fetchProxyList()

  // 检查是否需要自动恢复隧道
  const shouldRestore = localStorage.getItem('autoRestoreTunnels') === 'true'
  const isAutoStart = route.query.autostart === 'true'
  
  console.log('Should restore:', shouldRestore)
  console.log('Is autostart:', isAutoStart)
  console.log('Route query:', route.query)

  if (shouldRestore && isAutoStart) {
    // 等待隧道列表加载完成
    await nextTick()
    // 添加额外延迟确保一切就绪
    setTimeout(() => {
      if (tunnels.value.length > 0) {
        restoreTunnels()
      } else {
        console.log('No tunnels available yet, waiting for tunnels')
        // 如果隧道列表为空，监听隧道列表变化
        const unwatch = watch(tunnels, (newTunnels) => {
          if (newTunnels.length > 0) {
            restoreTunnels()
            unwatch() // 停止监听
          }
        })
      }
    }, 2000)
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

  // 监听隧道状态变化
  watch(tunnels, () => {
    saveTunnelStates()
  }, { deep: true })

  // 组件卸载时清理定时器
  onUnmounted(() => {
    clearInterval(statusCheckInterval)
    clearInterval(listRefreshInterval)
    clearInterval(checkInterval)
  })

  // 在组件卸载时保存状态
  onUnmounted(() => {
    saveTunnelStates()
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
                  控制隧道启动/停止</n-tooltip>
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
