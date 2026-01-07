<script setup lang="ts">
import { ref, onMounted, onUnmounted, h, computed, nextTick, watch, inject, Ref } from 'vue'
import { NCard, NSpace, NSwitch, NButton, NButtonGroup, NTooltip, useMessage, useNotification, useLoadingBar, NGrid, NGridItem, NText, NTag, NSkeleton, NScrollbar, NIcon, NDropdown, useDialog, NMessageProvider, NInput, NStatistic, NList, NListItem } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import axios from 'axios'
import { useLinkTunnelsStore } from '@/stores/linkTunnels'
import { listen } from '@tauri-apps/api/event'
import { useRoute } from 'vue-router'
import Cookies from '@/utils/cookies'

import frpApiGetUserProxies from '@/requests/frpApi/frpApiGetUserProxies';
import frpApiRemoveProxy from '@/requests/frpApi/frpApiRemoveProxy';
import frpApiEditProxy from '@/requests/frpApi/frpApiEditProxy';
import frpApiForceOff from '@/requests/frpApi/frpApiForceOff';
import frpApiChangeProxy from '@/requests/frpApi/frpApiChangeProxy'
import { Refresh, BuildOutline, CreateOutline, InformationOutline, TrashOutline, RefreshOutline, CloudOfflineOutline, DocumentOutline, CopyOutline } from '@vicons/ionicons5'

// 导入现有组件
import Edit from '@/components/ManageProxies/Edit.vue';
import Infomation from '@/components/ManageProxies/Infomation.vue';
import GetConf from '@/components/ManageProxies/GetConf.vue';
import Menu from '@/components/ManageProxies/Menu.vue';
import copy from 'copy-to-clipboard';


// 扩展Window接口，添加全局属性
declare global {
  interface Window {
    __tunnelsRestoreAttempted?: boolean;
  }
}

const message = useMessage();
const notification = useNotification();
const dialog = useDialog()
const tunnels = ref<any[]>([])
const searchQuery = ref('')
const filterType = ref<'all' | 'tcp' | 'udp'>('all')

const filteredTunnels = computed(() => {
  // 根据类型过滤
  let list = filterType.value === 'all' ? tunnels.value : tunnels.value.filter(t => t.type.toLowerCase() === filterType.value)

  // 文本搜索
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return list

  return tunnels.value.filter(t => {
    const idStr = String(t.id)
    const name = (t.name || '').toLowerCase()
    const local = (t.local || '').toLowerCase()
    const remote = (t.remote || '').toLowerCase()

    if (idStr.includes(q) || name.includes(q) || local.includes(q) || remote.includes(q)) {
      return true
    }

    // 端口号匹配（本地或远程）
    const num = Number(q)
    if (!isNaN(num)) {
      const localPort = parseInt(t.local.split(':').pop())
      let remotePort = t.remotePort || -1
      if (remotePort === -1 && t.remote.includes(':')) {
        remotePort = parseInt(t.remote.split(':').pop())
      }
      return localPort === num || remotePort === num
    }

    return false
  })
})
const loading = ref(false)
const loadingTunnels = ref<Set<string>>(new Set())
const loadingBar = useLoadingBar();

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

const startTunnel = async (tunnel: any) => {
  // 启动前通知日志页面注册监听器
  window.dispatchEvent(new CustomEvent('setup-tunnel-listener', { detail: { tunnelId: tunnel.id.toString() } }));
  console.log(`开始启动隧道 ${tunnel.id}`)

  // 如果已经在运行，不重复启动
  if (tunnel.status === 'running') {
    message.info('隧道已经在运行中')
    return
  }

  // 设置加载状态
  loadingTunnels.value.add(tunnel.id.toString())

  // 设置状态为启动中
  tunnel.status = 'starting'

  // 发送事件到系统日志，但使用prepare类型
  await invoke('emit_event', {
    event: 'tunnel-event',
    payload: {
      type: 'prepare',
      tunnelId: tunnel.id.toString(),
      tunnelName: tunnel.name
    }
  })

  // 获取token
  const token = userInfo?.value?.token ?? (localStorage.getItem('authorization') || Cookies.get('authorization') || '')
  if (!token) {
    message.error('未找到用户token，请重新登录')
    loadingTunnels.value.delete(tunnel.id.toString()) // 移除加载状态
    return
  }

  try {
    // 等待日志响应
    const result = await new Promise<{ success: boolean, message: string }>((resolve) => {
      let timeout: any = null;
      let resolved = false;
      // 定义成功回调
      const successEventListener = (event: any) => {
        if (resolved) return;
        resolved = true;
        const msg = event.detail?.message || '隧道启动成功'
        console.log(`收到隧道${tunnel.id}成功事件:`, msg)
        resolve({ success: true, message: msg })
        if (timeout) clearTimeout(timeout)
        window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
      }
      // 监听成功事件
      window.addEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
      // 设置25秒超时
      timeout = setTimeout(async () => {
        if (resolved) return;
        resolved = true;
        console.log(`隧道${tunnel.id}启动检查超时`)
        // 超时后检查状态，但不再判定为 success，只判定为失败
        try {
          const isRunning = await invoke('check_frpc_status', { id: tunnel.id.toString() })
          if (isRunning) {
            // 进程已运行但无日志事件，判定为失败（不弹窗）
            resolve({ success: false, message: '进程已运行但未收到日志事件，启动结果未知' })
            message.warning('若要反馈问题，请勿截图此弹窗，请向技术支持提供报错日志。')
          } else {
            resolve({ success: false, message: '启动超时，请检查日志' })
            message.warning('若要反馈问题，请勿截图此弹窗，请向技术支持提供报错日志。')
          }
        } catch (e) {
          resolve({ success: false, message: `启动超时: ${e}` })
          message.warning('若要反馈问题，请勿截图此弹窗，请向技术支持提供报错日志。')
        }
        window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
      }, 25000)
      // 发送启动事件
      invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'start',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      }).catch(e => console.error(`发送启动事件失败:`, e))
      // 启动隧道
      console.log(`调用start_frpc_instance启动隧道${tunnel.id}`)
      invoke('start_frpc_instance', {
        id: tunnel.id.toString(),
        token: token,
        tunnelId: tunnel.id.toString(),
        logColors: true,
        enableLog: true,
        logUser: userInfo?.value?.username || ''
      }).catch((error) => {
        if (resolved) return;
        resolved = true;
        console.error(`启动隧道 #${tunnel.id} 失败:`, error)
        if (timeout) clearTimeout(timeout)
        window.removeEventListener(`tunnel-${tunnel.id}-success`, successEventListener)
        resolve({ success: false, message: String(error) })
      })
    })

    console.log(`隧道${tunnel.id}启动结果:`, result)

    // 发送事件到系统日志，但不使用success类型，避免生成额外日志
    if (result.success) {
      // 不再发送success类型事件，避免生成重复日志
      // 成功日志已经通过隧道日志直接显示
      await invoke('emit_event', { event: 'tunnel-event', payload: { type: 'success', tunnelId: tunnel.id.toString(), tunnelName: tunnel.name } })
      message.success(`隧道 #${tunnel.id} ${tunnel.name} 启动成功`)
      // 新增：推送通知和系统通知
      notification.success({
        title: `隧道 #${tunnel.id} ${tunnel.name} 启动成功`,
        description: `隧道启动成功！连接地址: ${tunnel.remote}`,
        content: () => h('div', [
          h(NButton, {
            type: 'success',
            text: true,
            onClick: () => copyToClipboard(tunnel.remote)
          }, '复制链接地址')
        ]),
        duration: 5000
      })
      await sendNotification({ title: `隧道 #${tunnel.id} ${tunnel.name} 启动成功`, body: `请使用链接地址：\n${tunnel.remote}` })

      tunnel.status = 'running'
      saveTunnelStates()
    } else {
      await invoke('emit_event', {
        event: 'tunnel-event',
        payload: {
          type: 'error',
          tunnelId: tunnel.id.toString(),
          tunnelName: tunnel.name
        }
      })

      message.error(`隧道 ${tunnel.name} 启动失败: ${result.message}`)
      tunnel.status = 'stopped'
    }
  } catch (error) {
    console.error(`启动隧道异常:`, error)
    message.error(`启动隧道出错: ${error}`)
    tunnel.status = 'stopped'

    // 发送事件到系统日志
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'error',
        tunnelId: tunnel.id.toString(),
        tunnelName: tunnel.name
      }
    })
  } finally {
    // 清除加载状态
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

  // 记录保存状态到本地存储
  localStorage.setItem('tunnelStates', JSON.stringify(states))

  // 记录到系统日志
  const runningCount = Object.keys(states).length
  // if (runningCount > 0) {
  //   invoke('emit_event', {
  //     event: 'log',
  //     payload: {
  //       message: `已保存${runningCount}个运行中隧道的状态: ${Object.keys(states).join(', ')}`
  //     }
  //   })
  // }
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
  console.log('开始恢复隧道');

  // 发布恢复隧道日志
  await invoke('emit_event', {
    event: 'log',
    payload: {
      message: '开始恢复隧道...'
    }
  });

  // 检查是否是自动启动模式（仅作为日志，不阻止恢复）
  const isAutoStartMode = localStorage.getItem('appStartedByAutostart') === 'true' || route.query.autostart === 'true';
  if (!isAutoStartMode) {
    console.log('当前不是自动启动模式，但仍然继续恢复隧道');
  }

  // 获取保存的隧道状态
  const savedStates = localStorage.getItem('tunnelStates');
  if (savedStates) {
    const states = JSON.parse(savedStates);
    const tunnelsToRestore = tunnels.value.filter(tunnel => states[tunnel.id] === 'running');

    console.log('找到需要恢复的隧道:', tunnelsToRestore.map(t => t.id));

    // 添加系统事件日志
    await invoke('emit_event', {
      event: 'log',
      payload: {
        message: `找到${tunnelsToRestore.length}个需要恢复的隧道: ${tunnelsToRestore.map(t => t.id).join(', ')}`
      }
    });

    if (tunnelsToRestore.length > 0) {
      const delayBetweenTunnels = 5000; // 5秒间隔

      for (let i = 0; i < tunnelsToRestore.length; i++) {
        const tunnel = tunnelsToRestore[i];
        console.log(`准备启动隧道 ${i + 1}/${tunnelsToRestore.length}: ${tunnel.id}`);

        await invoke('emit_event', {
          event: 'log',
          payload: {
            message: `准备恢复隧道 ${i + 1}/${tunnelsToRestore.length}: ${tunnel.id} ${tunnel.name}`
          }
        });

        // 尝试启动隧道
        await startTunnel(tunnel);

        // 除了最后一个隧道，每个隧道启动后等待一段时间
        if (i < tunnelsToRestore.length - 1) {
          console.log(`等待${delayBetweenTunnels / 1000}秒后启动下一个隧道...`);
          await new Promise(resolve => setTimeout(resolve, delayBetweenTunnels));
        }
      }

      await invoke('emit_event', {
        event: 'log',
        payload: {
          message: `已尝试恢复所有隧道`
        }
      });
    }
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

const changeUserProxyAsync = async (proxyId: bigint, method: boolean) => {
  loadingBar.start();
  try {
    const res = await frpApiChangeProxy({
      proxy_id: proxyId,
      proxy_do: method,
    });

    if (res.flag) {
      loadingBar.finish();
      fetchProxyList();
      message.success(`隧道 #${proxyId}  已${method ? '启用' : '禁用'}`);
      return true;
    } else {
      message.info(res.msg);
      loadingBar.error();
    }
  } catch (error) {
    if (error instanceof Error) {
      message.error(error.message);
      loadingBar.error();
    }
  }
  return false;
};

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
    case 'stateChange': {
      // 切换隧道状态
      changeUserProxyAsync(row.id, !row.apiStatus);
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
        content: () => h(NMessageProvider, null, () => h(Edit, {
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
        })),
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

const toggleTunnel = async (tunnel: any) => {
  if (tunnel.status === 'running') {
    await stopTunnel(tunnel);
  } else {
    await startTunnel(tunnel);
  }
}

const stopTunnel = async (tunnel: any) => {
  console.log(`停止隧道 ${tunnel.id}`);

  // 如果已经停止，不重复操作
  if (tunnel.status === 'stopped') {
    message.info('隧道已经停止');
    return;
  }

  try {
    loadingTunnels.value.add(tunnel.id.toString());

    // 发送停止事件
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'stop',
        tunnelId: tunnel.id.toString(),
        tunnelName: tunnel.name
      }
    });

    // 停止隧道实例
    await invoke('stop_frpc_instance', { id: tunnel.id.toString() });

    // 更新状态
    tunnel.status = 'stopped';
    tunnel.apiOnline = false;
    linkTunnelsStore.removeLinkTunnel(tunnel.id.toString());

    // 保存状态
    saveTunnelStates();

    message.info(`隧道 ${tunnel.name} 已停止运行`);

    // 发送通知
    await sendNotification({
      title: `隧道 #${tunnel.id} ${tunnel.name} 已停止`,
      body: '隧道已成功停止运行'
    });

  } catch (error) {
    console.error(`停止隧道 ${tunnel.id} 失败:`, error);
    message.error(`停止隧道失败: ${error}`);

    // 检查实际状态
    await checkTunnelStatus(tunnel);
  } finally {
    loadingTunnels.value.delete(tunnel.id.toString());
  }
}

onMounted(async () => {
  console.log('ProxyList mounted')
  await requestNotificationPermission()
  await fetchProxyList()

  // 监听自动启动模式下的隧道恢复
  const isAutoStartMode = localStorage.getItem('appStartedByAutostart') === 'true' || route.query.autostart === 'true'
  const autoRestoreTunnels = localStorage.getItem('autoRestoreTunnels') === 'true'

  console.log('隧道组件挂载，自动恢复状态:', {
    isAutoStartMode,
    autoRestoreTunnels,
    hasToken: !!userInfo?.value?.token,
    alreadyAttempted: window.__tunnelsRestoreAttempted
  })

  // 添加全局恢复隧道事件监听器
  const restoreTunnelsListener = () => {
    console.log('收到全局恢复隧道事件');
    // 只在未尝试过的情况下恢复
    if (autoRestoreTunnels && !window.__tunnelsRestoreAttempted) {
      window.__tunnelsRestoreAttempted = true;
      restoreTunnels();
    } else {
      console.log('已经尝试过恢复隧道，忽略此事件');
    }
  };

  window.addEventListener('global-restore-tunnels', restoreTunnelsListener);

  // 检查是否需要自动恢复隧道（确保只执行一次）
  if (isAutoStartMode && autoRestoreTunnels && !window.__tunnelsRestoreAttempted) {
    console.log('自动启动模式，首次恢复隧道');
    window.__tunnelsRestoreAttempted = true;
    await restoreTunnels();
  } else if (window.__tunnelsRestoreAttempted) {
    console.log('已经尝试过恢复隧道，不再重复执行');
  }

  // 添加监听器清理函数
  onUnmounted(() => {
    window.removeEventListener('global-restore-tunnels', restoreTunnelsListener);
  });

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
    <n-space vertical style="height: 100%;margin-bottom: 8px;">
      <n-h2 style="margin-bottom: 3px;">隧道管理</n-h2>
      <n-skeleton v-if="loading" :height="3" />
      <n-card v-else-if="getLinkOnlyTunnels.size > 0" title="外部快速启动隧道">
        <n-list bordered hoverable>
          <n-list-item v-for="tunnelId in getLinkOnlyTunnels" :key="tunnelId">
            <n-space justify="space-between" align="center" style="width: 100%;">
              <n-space vertical :size="2">
                <n-text strong>外部隧道 #{{ tunnelId }}</n-text>
                <n-text depth="3" style="font-size: 12px;">快速启动 · 非本账号</n-text>
              </n-space>
              <n-button type="error" size="small" @click="stopExternalTunnel(tunnelId)">停止</n-button>
            </n-space>
          </n-list-item>
        </n-list>
      </n-card>

      <n-card>
        <n-space justify="space-between" align="center" wrap>
          <!-- 左侧：搜索 + 类型按钮 -->
          <n-space align="center" :size="8">
            <n-input v-model:value="searchQuery" placeholder="输入ID/名称/端口筛选" clearable style="max-width: 240px;" />
            <n-button-group>
              <n-button :type="filterType==='all' ? 'primary' : 'default'" @click="filterType='all'">全部</n-button>
              <n-button :type="filterType==='tcp' ? 'primary' : 'default'" @click="filterType='tcp'">TCP</n-button>
              <n-button :type="filterType==='udp' ? 'primary' : 'default'" @click="filterType='udp'">UDP</n-button>
            </n-button-group>
            <n-button @click="fetchProxyList" :loading="loading" >
              <template #icon>
                <n-icon v-if="!loading" size="18">
                  <Refresh />
                </n-icon>
              </template>
              刷新
            </n-button>
          </n-space>
          <!-- 右侧：刷新按钮 + 统计 -->
          <n-space align="center" :size="12">
            
            <n-statistic label="隧道(已用/总计)" :value="userInfo?.used || 0" :loading="loading">
              <template #suffix>/ {{ userInfo?.proxies || 0 }}</template>
            </n-statistic>
          </n-space>
        </n-space>
      </n-card>

      <n-skeleton v-if="loading" :height="3" />
      <n-grid v-else :cols="2" :x-gap="12" :y-gap="12">
        <n-grid-item v-for="tunnel in filteredTunnels" :key="tunnel.id" style="display: flex;">
          <n-card :title="'隧道 #' + tunnel.id + ' ' + tunnel.name" :bordered="false" size="small"
            :class="{ 'disabled-card': !canStartTunnel(tunnel) }" style="flex: 1; height: 100%;">
            <template #header-extra>
              <n-space>
                <n-tag v-if="linkTunnelsStore.linkLaunchedTunnels.has(tunnel.id.toString())" type="info" size="small">
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
                    <n-switch :value="tunnel.status === 'running'" @update:value="() => toggleTunnel(tunnel)"
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
