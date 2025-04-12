<script setup lang="ts">
import { ref, onMounted, onUnmounted,h, computed, nextTick, watch,inject,Ref } from 'vue'
import { NCard, NSpace, NSwitch, NButton, NTooltip, useMessage, useNotification, NGrid, NGridItem, NText, NTag, NSkeleton, NScrollbar, NIcon, NDropdown, useDialog, NMessageProvider } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import axios from 'axios'
import { useLinkTunnelsStore } from '@/stores/linkTunnels'
import { listen } from '@tauri-apps/api/event'
import { useRoute } from 'vue-router'

import frpApiGetUserProxies from '@/requests/frpApi/frpApiGetUserProxies';
import frpApiRemoveProxy from '@/requests/frpApi/frpApiRemoveProxy';
import frpApiEditProxy from '@/requests/frpApi/frpApiEditProxy';
import frpApiForceOff from '@/requests/frpApi/frpApiForceOff';
import { BuildOutline, CreateOutline, InformationOutline, TrashOutline, RefreshOutline, CloudOfflineOutline, DocumentOutline, CopyOutline } from '@vicons/ionicons5'

// 导入现有组件
import Edit from '@/components/ManageProxies/Edit.vue';
import Infomation from '@/components/ManageProxies/Infomation.vue';
import GetConf from '@/components/ManageProxies/GetConf.vue';
import Menu from '@/components/ManageProxies/Menu.vue';
import copy from 'copy-to-clipboard';

const message = useMessage()
const notification = useNotification()
const dialog = useDialog()
const tunnels = ref<any[]>([])
const loading = ref(false)
const loadingTunnels = ref<Set<string>>(new Set())

const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');
const userInfo = userInfoObj?.userInfo;

const linkTunnelsStore = useLinkTunnelsStore()

const route = useRoute()

interface ExternalTunnel {
  id: string;
  status: 'running' | 'stopped';
}

const externalTunnels = ref<Map<string, ExternalTunnel>>(new Map())

const getUserToken = () => {
  return localStorage.getItem('userToken') || ''
}

const fetchProxyList = async () => {
  loading.value = true
  try {
    const response = await frpApiGetUserProxies();
    
    if (response.flag) {
      tunnels.value = response.data.list.map(proxy => {
        const local = `${proxy.localIp}:${proxy.localPort}`;
        
        let remote = proxy.connectAddress;
        if (proxy.proxyType !== 'http' && proxy.proxyType !== 'https' && !remote.includes(':')) {
          remote = `${remote}:${proxy.remotePort}`;
        }
        
        return {
          id: proxy.id,
          name: proxy.proxyName,
          type: proxy.proxyType,
          node: proxy.friendlyNode,
          nodeId: proxy.nid,
          local: local,
          remote: remote,
          status: 'stopped',
          apiStatus: proxy.status,
          apiOnline: proxy.online,
          useCompression: proxy.useCompression,
          useEncryption: proxy.useEncryption,
          domain: proxy.domain,
          remotePort: proxy.remotePort,
          custom: proxy.custom,
          autoTls: proxy.autoTls,
          forceHttps: proxy.forceHttps,
          proxyProtocolVersion: proxy.proxyProtocolVersion,
          extAddress: proxy.extAddress,
          nodeHostname: proxy.nodeHostname
        };
      });
      
      tunnels.value.sort((a, b) => a.id - b.id)

      loadTunnelStates()

      await Promise.all(tunnels.value.map(tunnel => checkTunnelStatus(tunnel)))
    } else {
      message.error(response.msg)
      tunnels.value = []
    }
  } catch (error) {
    console.error('获取用户隧道列表失败:', error)
    tunnels.value = []
    message.error('获取隧道列表失败，请稍后重试')
  } finally {
    loading.value = false
  }
}

watch(() => userInfo?.value?.token, (newToken) => {
  if (!newToken) {
    tunnels.value = []
    loading.value = false
  }
}, { immediate: true })

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

const isSuccessLog = (log: string): boolean => {
  const successPatterns = [
    /start.*success/i,
    /启动.*成功/,
    /隧道.*成功/,
    /success/i,
    /connected/i,
    /running/i
  ]

  return successPatterns.some(pattern => pattern.test(log))
}

const checkTunnelStatus = async (tunnel: any) => {
  try {
    const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })
    tunnel.status = isRunning ? 'running' : 'stopped'
    if (!isRunning) {
      linkTunnelsStore.removeLinkTunnel(tunnel.id.toString())
    }
  } catch (e) {
    console.error('检查隧道状态失败:', e)
  }
}

const checkAllTunnelsStatus = () => {
  tunnels.value.forEach(tunnel => checkTunnelStatus(tunnel))
}

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
  const token = userInfo?.value?.token

  if (!token) {
    message.error('未获取到用户 token，请确保已登录')
    return
  }

  try {
    loadingTunnels.value.add(tunnel.id.toString())

    const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })

    if (isRunning) {
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
      tunnel.apiOnline = false
      linkTunnelsStore.removeLinkTunnel(tunnel.id.toString())
      message.info(`隧道 #${tunnel.id} ${tunnel.name} 已停止运行`)
      
      await checkTunnelStatus(tunnel)
      saveTunnelStates()
    } else {
      console.log(`隧道 ${tunnel.id} 准备执行启动命令...`)
      
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'prepare',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })
      
      await new Promise(resolve => setTimeout(resolve, 2000))
      
      console.log(`监听器设置完成，开始启动隧道 ${tunnel.id}...`)
      
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'start',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })
      
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      message.loading('正在启动隧道', {duration: 1000})
      let logMessage = ''
      
      const logResult = await new Promise<{success: boolean, message: string}>((resolve) => {
        console.log(`等待隧道 ${tunnel.id} 的日志响应...`)
        
        const successEventListener = (event: any) => {
          console.log(`接收到隧道 ${tunnel.id} 的成功事件:`, event)
          clearTimeout(timeout)
          window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
          resolve({success: true, message: event.detail.message})
        }
        window.addEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
        
        console.log(`开始监听隧道 ${tunnel.id} 的日志...`)
        
        const logPromise = listen(`frpc-log-${tunnel.id}`, (event: any) => {
          const log = event.payload.message
          logMessage = log
          
          if (isSuccessLog(log) || 
              log.includes('start proxy success') || 
              log.includes('start tunnel success') ||
              log.includes('隧道启动成功')) {
            clearTimeout(timeout)
            logPromise.then(unlisten => unlisten())
            window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
            resolve({success: true, message: log})
          } else if (log.includes('启动失败') || log.includes('failed') || log.includes('error')) {
            clearTimeout(timeout)
            logPromise.then(unlisten => unlisten())
            window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
            const errorMatch = log.match(/启动失败:\s*(.+)/)
            const errorMessage = errorMatch ? errorMatch[1] : log
            resolve({success: false, message: errorMessage})
          }
        })
        
        const timeout = setTimeout(() => {
          console.log(`隧道 ${tunnel.id} 启动超时，检查隧道状态...`)
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
        
        setTimeout(() => {
          console.log(`执行启动命令：隧道 ${tunnel.id}...`)
          invoke('start_frpc_instance', {
            id: tunnel.id.toString(),
            token: token,
            tunnelId: tunnel.id.toString()
          }).catch((error) => {
            console.error(`启动隧道 ${tunnel.id} 失败:`, error)
            clearTimeout(timeout)
            logPromise.then(unlisten => unlisten())
            resolve({success: false, message: String(error)})
          })
        }, 500)
      })

      if (logResult.success) {
        tunnel.status = 'running'
        saveTunnelStates()
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
        tunnel.apiOnline = false
        saveTunnelStates()
        message.error(`隧道启动失败: ${logResult.message}`)
        await sendNotification({ title: `隧道 #${tunnel.id}  ${tunnel.name} 启动失败`, body: logResult.message })
      }
    }
  } catch (e) {
    tunnel.status = 'stopped'
    tunnel.apiOnline = false
    saveTunnelStates()
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'error',
        tunnelId: tunnel.id.toString(),
        tunnelName: tunnel.name
      }
    })

    message.error(`操作失败: ${e}`)
    await checkTunnelStatus(tunnel)
  } finally {
    loadingTunnels.value.delete(tunnel.id.toString())
  }
}

const saveTunnelStates = () => {
  const states: Record<string, string> = {}
  tunnels.value.forEach(tunnel => {
    if (tunnel.status === 'running') {
      states[tunnel.id] = tunnel.status
    }
  })
  console.log('Saving current running tunnel states:', states)
  localStorage.setItem('tunnelStates', JSON.stringify(states))
}

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

const restoreTunnels = async () => {
  const savedStates = localStorage.getItem('tunnelStates')
  console.log('Attempting to restore tunnels, saved states:', savedStates)
  
  if (savedStates) {
    const states = JSON.parse(savedStates)
    const tunnelsToRestore = tunnels.value.filter(tunnel => states[tunnel.id] === 'running')
    
    console.log('Found tunnels to restore:', tunnelsToRestore.map(t => t.id))
    
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    for (const tunnel of tunnelsToRestore) {
      try {
        console.log('Starting tunnel:', tunnel.id)
        await new Promise(resolve => setTimeout(resolve, 2000))
        await toggleTunnel(tunnel)
        await new Promise(resolve => setTimeout(resolve, 3000))
        console.log('Tunnel started:', tunnel.id)
      } catch (e) {
        console.error(`自动启动隧道 ${tunnel.id} 失败:`, e)
        continue
      }
    }
    
    saveTunnelStates()
  }
}

const getLinkOnlyTunnels = computed(() => {
  const result = new Set(linkTunnelsStore.linkLaunchedTunnels)
  tunnels.value.forEach(tunnel => {
    if (result.has(tunnel.id.toString())) {
      result.delete(tunnel.id.toString())
    }
  })
  
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

const stopExternalTunnel = async (tunnelId: string) => {
  try {
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

const canStartTunnel = (tunnel: any) => {
  if (tunnel.status === 'running') {
    return true;
  }
  
  return tunnel.apiStatus === true && tunnel.apiOnline === false;
}

const getDisabledReason = (tunnel: any) => {
  if (tunnel.status === 'running') {
    return '';
  }
  
  if (tunnel.apiStatus !== true) {
    return '隧道已被禁用';
  }
  if (tunnel.apiOnline === true) {
    return '隧道已在其它地方启动';
  }
  return '';
}

const renderIcon = (icon: any) => {
  return () => h(NIcon, null, { default: () => h(icon) })
}

const handleOnSelected = (key: string, row: any) => {
  switch (String(key)) {
    case 'refreshState': {
      // 刷新隧道状态
      fetchProxyList();
      break;
    }
    case 'getConf': {
      const content = ref<string>();

      const dx = dialog.info({
        showIcon: false,
        blockScroll: true,
        title: `获取配置 隧道 ${row.name}`,
        content: () => h(NMessageProvider, null, {
          default: () => h(GetConf, {
            proxy: {
              id: row.id,
              proxyName: row.name,
              proxyType: row.type,
              friendlyNode: row.node,
              localIp: row.local.split(':')[0],
              localPort: parseInt(row.local.split(':')[1]),
              nid: row.nodeId,
              connectAddress: row.remote,
              remotePort: row.remotePort || -1,
              status: row.apiStatus,
              online: row.apiOnline,
              useCompression: row.useCompression || false,
              useEncryption: row.useEncryption || false,
              domain: row.domain || '[]',
              custom: row.custom || '',
              autoTls: row.autoTls || 'false',
              forceHttps: row.forceHttps || false,
              proxyProtocolVersion: row.proxyProtocolVersion || false,
              extAddress: row.extAddress || [],
              nodeHostname: row.nodeHostname || '',
              lastLogin: BigInt(0),
              lastupdate: BigInt(0)
            },
            token: userInfo?.value?.token ?? '',
            fallback(x: string) {
              if (dx.positiveButtonProps != null) {
                dx.positiveButtonProps.disabled = x === null;
              }
              content.value = x;
            },
          }),
        }),
        onPositiveClick(e) {
          e.preventDefault();
          if (content.value != null) {
            copy(content.value);
            message.info('复制成功');
          }
        },
        positiveText: '复制',
        positiveButtonProps: {
          renderIcon: renderIcon(CopyOutline),
          disabled: true,
        },
      });
      break;
    }
    case 'getInfo': {
      dialog.info({
        showIcon: false,
        blockScroll: true,
        content: () => h(NMessageProvider, null, {
          default: () => h(Infomation, {
            proxy: {
              id: row.id,
              proxyName: row.name,
              proxyType: row.type,
              friendlyNode: row.node,
              localIp: row.local.split(':')[0],
              localPort: parseInt(row.local.split(':')[1]),
              nid: row.nodeId,
              connectAddress: row.remote,
              remotePort: row.remotePort || -1,
              status: row.apiStatus,
              online: row.apiOnline,
              useCompression: row.useCompression || false,
              useEncryption: row.useEncryption || false,
              domain: row.domain || '[]',
              custom: row.custom || '',
              autoTls: row.autoTls || 'false',
              forceHttps: row.forceHttps || false,
              proxyProtocolVersion: row.proxyProtocolVersion || false,
              extAddress: row.extAddress || [],
              nodeHostname: row.nodeHostname || '',
              lastLogin: BigInt(0),
              lastupdate: BigInt(0)
            },
            copyFallback(url) {
              copy(url);
              message.info('复制成功');
            },
          }),
        }),
        title: `隧道 ${row.name} 的信息`,
      });
      break;
    }
    case 'deleteProxy': {
      dialog.error({
        title: `移除隧道 ${row.name}`,
        content: '真的要移除该隧道吗?',
        positiveText: '确认',
        negativeText: '取消',
        onPositiveClick(e) {
          e.preventDefault();
          frpApiRemoveProxy({ proxy_id: row.id }).then((res) => {
            if (res.flag) {
              fetchProxyList();
            } else {
              message.error(res.msg);
            }
          });
        },
      });
      break;
    }
    case 'editConf': {
      const state = ref<boolean>(false);
      const d = dialog.info({
        blockScroll: true,
        maskClosable: false,
        showIcon: false,
        title: `编辑隧道 ${row.name}`,
        content: () => h(NMessageProvider, null, {
          default: () => h(Edit, {
            isEditMode: true,
            editConfig: {
              id: row.id,
              proxyName: row.name,
              proxyType: row.type,
              friendlyNode: row.node,
              localIp: row.local.split(':')[0],
              localPort: parseInt(row.local.split(':')[1]),
              nid: row.nodeId,
              connectAddress: row.remote,
              remotePort: row.remotePort || -1,
              status: row.apiStatus,
              online: row.apiOnline,
              useCompression: row.useCompression || false,
              useEncryption: row.useEncryption || false,
              domain: row.domain || '[]',
              custom: row.custom || '',
              autoTls: row.autoTls || 'false',
              forceHttps: row.forceHttps || false,
              proxyProtocolVersion: row.proxyProtocolVersion || false,
              extAddress: row.extAddress || [],
              nodeHostname: row.nodeHostname || '',
              lastLogin: BigInt(0),
              lastupdate: BigInt(0)
            },
            nodeConfig: undefined,
            watchDog: state,
            fallback(success, body) {
              d.loading = true;
              if (success && body !== undefined) {
                frpApiEditProxy(body)
                  .then((res) => {
                    if (res.flag) {
                      d?.destroy();
                      fetchProxyList();
                    } else {
                      message.error(res.msg);
                    }
                  })
                  .catch((error) => {
                    message.error(error.message || '未知错误');
                  });
              } else {
                d.loading = false;
              }
              state.value = false;
            },
          }),
        }),
        positiveText: '保存',
        negativeText: '取消',
        onPositiveClick(e) {
          e.preventDefault();
          d.loading = true;
          state.value = true;
          return false;
        },
      });
      break;
    }
    case 'forceOff': {
      dialog.warning({
        title: `强制下线隧道 ${row.name}`,
        content: `真的要下线隧道 ${row.name}嘛？\n注: 部分节点无法远程下线。`,
        negativeText: '取消',
        positiveText: '下线隧道',
        onPositiveClick(e) {
          e.preventDefault();
          forceOffTunnel(row);
        },
      });
      break;
    }
    default:
      break;
  }
};

const renderMenuDropdown = (tunnel: any) => {
  return h(Menu, {
    proxy: {
      id: tunnel.id,
      proxyName: tunnel.name,
      proxyType: tunnel.type,
      friendlyNode: tunnel.node, 
      localIp: tunnel.local.split(':')[0],
      localPort: parseInt(tunnel.local.split(':')[1]),
      nid: tunnel.nodeId,
      connectAddress: tunnel.remote,
      remotePort: tunnel.remotePort || -1,
      status: tunnel.apiStatus,
      online: tunnel.apiOnline,
      useCompression: tunnel.useCompression || false,
      useEncryption: tunnel.useEncryption || false,
      domain: tunnel.domain || '[]',
      custom: tunnel.custom || '',
      autoTls: tunnel.autoTls || 'false',
      forceHttps: tunnel.forceHttps || false,
      proxyProtocolVersion: tunnel.proxyProtocolVersion || false,
      extAddress: tunnel.extAddress || [],
      nodeHostname: tunnel.nodeHostname || '',
      lastLogin: BigInt(0),
      lastupdate: BigInt(0)
    },
    fallback: (key) => handleOnSelected(key, tunnel),
    btnStyle: ''
  });
};

const forceOffTunnel = (row: any) => {
  frpApiForceOff({ proxy_id: row.id }).then((res) => {
    if (res.flag) {
      fetchProxyList();
      message.info(`隧道 ${row.name} 下线成功!`);
    } else {
      message.error(res.msg);
    }
  });
};

onMounted(async () => {
  console.log('ProxyList mounted')
  await requestNotificationPermission()
  await fetchProxyList()

  const shouldRestore = localStorage.getItem('autoRestoreTunnels') === 'true'
  const isAutoStart = route.query.autostart === 'true' || localStorage.getItem('appStartedByAutostart') === 'true'
  
  console.log('Should restore:', shouldRestore)
  console.log('Is autostart:', isAutoStart)
  console.log('Route query:', route.query)

  // 清除自启动标记以避免非自启动下次运行时错误触发
  if (localStorage.getItem('appStartedByAutostart') === 'true') {
    localStorage.removeItem('appStartedByAutostart')
  }

  if (shouldRestore && isAutoStart) {
    await nextTick()
    setTimeout(() => {
      if (tunnels.value.length > 0) {
        restoreTunnels()
      } else {
        console.log('No tunnels available yet, waiting for tunnels')
        const unwatch = watch(tunnels, (newTunnels) => {
          if (newTunnels.length > 0) {
            restoreTunnels()
            unwatch()
          }
        })
      }
    }, 2000)
  }

  const statusCheckInterval = setInterval(checkAllTunnelsStatus, 5000)
  const listRefreshInterval = setInterval(fetchProxyList, 45000)

  const checkInterval = setInterval(() => {
    externalTunnels.value.forEach((_, tunnelId) => {
      checkExternalTunnelStatus(tunnelId)
    })
  }, 5000)

  watch(tunnels, () => {
    saveTunnelStates()
  }, { deep: true })

  onUnmounted(() => {
    clearInterval(statusCheckInterval)
    clearInterval(listRefreshInterval)
    clearInterval(checkInterval)
  })

  onUnmounted(() => {
    saveTunnelStates()
  })
})
</script>

<template>
  <n-scrollbar>
    <n-space vertical>
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
            :class="{ 'disabled-card': !canStartTunnel(tunnel) }"
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

             

             
            </n-space>
            <template #footer>
              <n-space justify="end">
                <component :is="renderMenuDropdown(tunnel)"></component>
                <n-tooltip :disabled="canStartTunnel(tunnel)" trigger="hover">
                  <template #trigger>
                  <n-switch :value="tunnel.status === 'running'" 
                            @update:value="() => toggleTunnel(tunnel)"
                            :loading="loadingTunnels.has(tunnel.id.toString())"
                            :disabled="loadingTunnels.has(tunnel.id.toString()) || !canStartTunnel(tunnel)" />
                </template>
                {{ getDisabledReason(tunnel) }}
              </n-tooltip>
              </n-space>
            </template>
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

.disabled-card {
  opacity: 0.7;
  filter: grayscale(50%);
  border: 1px dashed #d9d9d9;
}

.disabled-card:hover {
  box-shadow: 0 2px 12px 0 rgba(255, 0, 0, 0.1);
}
</style>
