<script lang="ts" setup>
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useDialog, /*NText,*/ useNotification, NButton, useMessage,NIcon } from 'naive-ui';
import { onMounted, h, onUnmounted, ref,provide } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { RouterLink, useRouter ,useRoute} from 'vue-router';
import { onOpenUrl, getCurrent } from '@tauri-apps/plugin-deep-link'
import { useLinkTunnelsStore } from '@/stores/linkTunnels'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { Remove, Expand, Contract, Close } from '@vicons/ionicons5'
import { AxiosError } from 'axios';


import frpApiGetUserInfo from '@/requests/frpApi/frpApiGetUserInfo';

import Cookies from '@/utils/cookies';

const notification = useNotification();
const router = useRouter();
const route = useRoute();
const userInfo = ref<Struct.UserInfo>();

const errorMessage = ref<{
  statusCode: number;
  title: string;
}>({ statusCode: 500, title: '加载中' });
const loaded = ref<boolean>(false);

// 获取用户信息
const getUserInfo = () => {
  frpApiGetUserInfo()
    .then((res) => {
      console.log(res);
      if (res.flag) {
        console.log(res.data)
        userInfo.value = res.data;
        // 在这里提供用户信息，确保在数据加载后立即提供
        provide('userInfo', { userInfo, getUserInfo });
        setTimeout(() => {
          loaded.value = true;
        }, 250);
      } else {
        // 需要登录的情况
        sessionStorage.setItem('redirectPath', route.fullPath);
        Cookies.remove('authorization');
        router.push('/settings');
      }
    })
    .catch((res: AxiosError) => {
      errorMessage.value = {
        statusCode: Number(res.response?.status),
        title: res.response?.statusText ?? '发生了未知错误',
      };
      notification.error({
        content: res.message,
        meta: '这可能是由于服务器的问题，请稍后重试。',
        duration: 4500,
      });
      loaded.value = false;
    });
};
getUserInfo();



const isMaximized = ref(false)
const minimizeWindow = async () => {
  const window = await getCurrentWindow()
  await window.minimize()
}

const maximizeWindow = async () => {
  const window = await getCurrentWindow()
  if (await window.isMaximized()) {
    await window.unmaximize()
    isMaximized.value = false
  } else {
    await window.maximize()
    isMaximized.value = true
  }
}

const closeWindow = async () => {
  const window = await getCurrentWindow()
  await window.close()
}

// 监听窗口状态变化
onMounted(async () => {
  const window = await getCurrentWindow()
  isMaximized.value = await window.isMaximized()
  
  await window.onResized(() => {
    window.isMaximized().then(maximized => {
      isMaximized.value = maximized
    })
  })
})
const dialog = useDialog()
import './style.less';
const message = useMessage()

let unlistenNeedDownload: any = null

const currentVersion = ref('v0.1')

interface CplUpdate {
  title: string;
  latest: string;
  msg: string;
}

const getCurrentVersion = async () => {
  try {
    const version = await invoke('get_cpl_version')
    currentVersion.value = version as string
  } catch (e) {
    currentVersion.value = '获取失败'
    console.error('获取版本失败:', e)
  }
}
getCurrentVersion()

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

const handleHeaderDoubleClick = async () => {
  await maximizeWindow()
}

const linkTunnelsStore = useLinkTunnelsStore()
const processingLinks = ref(new Set<string>())

const isSuccessLog = (log: string): boolean => {
  const successPatterns = [
    /start.*success/i,          // 匹配 "start xxx success"
    /启动成功/,               // 匹配 "xxx启动xxx成功xxx"
  ]
  return successPatterns.some(pattern => pattern.test(log))
}

const handleDeepLink = async (url: string) => {
  const urlObj = new URL(url)
  console.log('deep link:', url);
  if (urlObj.protocol !== 'openfrp:') return
  // 直接解析 URL 字符串
  const match = url.match(/^openfrp:\/\/([^/?]+)/)
  if (!match) return
  
  const path = match[1]
  console.log('解析路径:', path)
  // 将窗口提升到最前方
  try {
    const appWindow = await getCurrentWindow()
    await appWindow.show()
    await appWindow.setFocus()
  } catch (e) {
    console.error('窗口提升失败:', e)
  }
  // 处理登录回调
  if (path === 'login') {
    const params = new URLSearchParams(urlObj.search)
    const code = params.get('code')
    
    if (code) {
      // 保存 token
      // localStorage.setItem('userToken', token)
      message.success('获取到登录码，您可以关闭刚刚打开的窗口了')
      urlObj.searchParams.delete('code')
      // 发送事件到日志系统
    
      router.push(`/oauth_callback?code=${code}`)
      
      // 可选：刷新页面或更新状态
      // window.location.reload()
      return
    } else {
      message.error('登录失败：未获取到 token')
      return
    }
    
  }

  // 处理登录回调
  if (path === 'start_proxy') {
  const params = new URLSearchParams(urlObj.search)
  const user = params.get('user')
  const proxyId = params.get('proxy')
  const remoteAddress = params.get('remote')
  const proxyName = params.get('name')

  if (!user || !proxyId || !remoteAddress || !proxyName) {
    await sendNotification({ title: '快速启动链接格式错误', body: '请检查链接格式是否正确' })
    console.error('链接格式错误')
    return
  }
  
  // 检查是否正在处理
  if (processingLinks.value.has(proxyId)) {
    console.log('隧道正在启动中，跳过:', proxyId)
    return
  }
  try {
    processingLinks.value.add(proxyId)
  
    // 发送启动事件到日志系统
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'start',
        tunnelId: proxyId,
        tunnelName: proxyName
      }
    })
  
    message.loading('正在启动隧道', { duration: 1000 })
  
    // 等待日志响应
    const logResult = await new Promise<{ success: boolean, message: string }>((resolve) => {
      const timeout = setTimeout(async () => {
        // 超时时检查隧道是否实际在运行
        try {
          const isRunning = await invoke('check_frpc_status', { id: proxyId })
          if (isRunning) {
            resolve({ success: true, message: '隧道已启动' })
          } else {
            resolve({ success: false, message: '启动超时，请检查日志。' })
          }
        } catch (e) {
          resolve({ success: false, message: '启动超时，请检查日志。' })
        }
      }, 5000)

      const logListener = listen(`frpc-log-${proxyId}`, (event: any) => {
        const log = event.payload.message

        if (isSuccessLog(log)) {
          clearTimeout(timeout)
          logListener.then(unlisten => unlisten())
          resolve({ success: true, message: log })
        } else if (log.includes('启动失败')) {
          clearTimeout(timeout)
          logListener.then(unlisten => unlisten())
          const errorMatch = log.match(/启动失败:\s*(.+)/)
          const errorMessage = errorMatch ? errorMatch[1] : log
          resolve({ success: false, message: errorMessage })
        }
      })

      // 先检查是否已经在运行
      invoke('check_frpc_status', { id: proxyId })
        .then(isRunning => {
          if (isRunning) {
            clearTimeout(timeout)
            logListener.then(unlisten => unlisten())
            resolve({ success: true, message: '隧道已在运行' })
            return
          }

          // 如果没有运行，则启动隧道
          return invoke('start_frpc_instance', {
            id: proxyId,
            token: user,
            tunnelId: proxyId,
            logColors: true,
            enableLog: true,
            logUser: user
          })
        })
        .catch((error) => {
          clearTimeout(timeout)
          logListener.then(unlisten => unlisten())
          resolve({ success: false, message: String(error) })
        })
    })

    await requestNotificationPermission()

    const copyToClipboard = async (text: string) => {
      try {
        await navigator.clipboard.writeText(text)
        message.success('已复制到剪贴板')
      } catch (err) {
        message.error('复制失败')
        console.error('复制失败:', err)
      }
    }

    if (logResult.success) {
      notification.success({
        title: `隧道 #${proxyId} ${proxyName} 启动成功`,
        description: `隧道启动成功！连接地址: ${remoteAddress}`,
        content: () => h('div', [
          h(NButton, {
            type: 'success',
            text: true,
            onClick: () => copyToClipboard(remoteAddress)
          }, '复制链接地址')
        ]),
        duration: 5000
      })
      await sendNotification({ title: `隧道 #${proxyId} ${proxyName} 启动成功`, body: `请使用链接地址：${remoteAddress}` })
      linkTunnelsStore.addLinkTunnel(proxyId)
      message.success('通过快速启动隧道成功')
      setTimeout(() => {
        message.info('您通过快速启动启动了一条隧道，您可在网页版或日志中查看链接地址与启动状态')
      }, 1000);
    } else {
      notification.error({
        title: `隧道 #${proxyId} ${proxyName} 启动失败`,
        content: logResult.message,
        duration: 5000
      })
      await sendNotification({ title: `隧道 #${proxyId} ${proxyName} 启动失败`, body: logResult.message })
      message.error(`启动隧道失败: ${logResult.message}`)
    }

  } catch (e) {
    // 发送错误事件到日志系统
    await invoke('emit_event', {
      event: 'tunnel-event',
      payload: {
        type: 'error',
        tunnelId: proxyId,
        tunnelName: proxyName
      }
    })
    await sendNotification({ title: `隧道 #${proxyId} 启动失败`, body: e as any })
    message.error(`启动隧道失败: ${e}`)

  } finally {
    processingLinks.value.delete(proxyId)
  }
    return
  }

  message.error('错误:未知链接操作')
}

// 创建事件清理函数的引用
let cleanupOpenUrl: (() => void) | null = null
let cleanupSecondInstance: (() => void) | null = null

onMounted(async () => {
  // 检查更新
  try {
    const update = await invoke('check_update') as CplUpdate
    if (update) {
      const notificationInstance = notification.info({
        title: update.title,
        description: `发现新版本 v${update.latest}\n${update.msg}`,
        content: () => h('div', [
          h(NButton, {
            text: true, type: 'primary',
            onClick: async () => {
              notificationInstance.destroy()
              try {
                await invoke('download_and_update')
              } catch (e) {
                notification.error({
                  title: '更新失败',
                  content: String(e)
                })
              }
            },
            style: 'margin-top: 8px;'
          }, '立即更新')
        ]),
        duration: 10000
      })
    }
  } catch (e) {
    console.error('检查更新失败:', e)
  }

  unlistenNeedDownload = await listen('need_download', async () => {
    const notificationInstance = notification.warning({
      title: '提示',
      description: '未检测到 frpc，是否前往设置页面下载？\n您必须下载 frpc 才能继续使用本程序。',
      duration: 8000,
      content: () => h(RouterLink, {
        to: { path: '/settings' },
        onClick: () => {
          notificationInstance.destroy();
          router.push('/settings');
        }
      }, '前往下载')
    })
  })

  // 检查 frpc
  try {
    const handle = await getCurrentWindow()
    if (await invoke('check_frpc_status', { id: "0" })) {
      await invoke('download_frpc', { app: handle })
    }
  } catch (e) {
    console.error('检查 frpc 失败:', e)
  }

  const appWindow = getCurrentWindow();

  appWindow.onCloseRequested(async () => {
    dialog.warning({
      title: '二次确认',
      content: '确认退出程序？',
      positiveText: '最小化到托盘',
      negativeText: '彻底退出',
      closeOnEsc: true,
      maskClosable: true,
      showIcon: true,
      closable: true,
      onPositiveClick: async () => {
        await appWindow.hide();
      },
      onNegativeClick: async () => {
        try {
          // 先结束所有隧道
          await invoke('kill_all_processes');
          // 然后退出程序
          await invoke('exit_app');
        } catch (e) {
          console.error(e);
        }
      }
    })
  });

  // 监听 frpc 更新成功事件
  await listen('frpc-update', (event) => {
    notification.success({
      title: 'Frpc 更新',
      content: event.payload as string,
      duration: 5000
    })
  })

  // 监听 frpc 更新失败事件
  await listen('frpc-update-error', (event) => {
    notification.error({
      title: 'Frpc 更新失败',
      content: event.payload as string,
      duration: 5000
    })
  })

  // 检查启动参数
  const urls = await getCurrent()
  if (urls && urls.length > 0) {
    await handleDeepLink(urls[0])
  }

  // 监听新的链接
  cleanupOpenUrl = await onOpenUrl((urls) => {
    if (urls.length > 0) {
      handleDeepLink(urls[0])
    }
  })

  // 监听来自第二个实例的参数
  cleanupSecondInstance = await listen('second-instance', (event: any) => {
    const args = event.payload as string[]
    for (const arg of args) {
      if (arg.startsWith('openfrp://')) {
        handleDeepLink(arg)
      }
    }
  })
})

onUnmounted(() => {
  if (unlistenNeedDownload) {
    unlistenNeedDownload()
  }
  if (cleanupOpenUrl) cleanupOpenUrl()
  if (cleanupSecondInstance) cleanupSecondInstance()
})

// 移除这里的 provide，因为我们已经在 getUserInfo 成功回调中提供了
// provide('userInfo', { userInfo, getUserInfo });
</script>

<template>
   <div class="header" data-tauri-drag-region @dblclick="handleHeaderDoubleClick">
    <div class="header-left" data-tauri-drag-region>
      <div class="header-logo" data-tauri-drag-region>
        <div style="margin-top: 6px">
          <svg fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 361.29 51.55">
            <path
              d="M99.65,156.91A29,29,0,0,1,89,155a25.58,25.58,0,0,1-8.53-5.44,25,25,0,0,1-5.61-8.17,26.89,26.89,0,0,1,0-20.45,25,25,0,0,1,5.61-8.17,25.65,25.65,0,0,1,8.5-5.44,30.65,30.65,0,0,1,21.38,0,25.18,25.18,0,0,1,8.5,5.4,24.47,24.47,0,0,1,5.58,8.2,27.68,27.68,0,0,1,0,20.56,24.65,24.65,0,0,1-5.58,8.17,25.18,25.18,0,0,1-8.5,5.4A29.13,29.13,0,0,1,99.65,156.91Zm0-6.41a20.41,20.41,0,0,0,7.73-1.44,18.36,18.36,0,0,0,6.2-4.07,18.89,18.89,0,0,0,4.1-6.15,19.84,19.84,0,0,0,1.48-7.71,19.3,19.3,0,0,0-5.58-13.82,18.14,18.14,0,0,0-6.2-4.1,20.41,20.41,0,0,0-7.73-1.44,20.69,20.69,0,0,0-7.82,1.44,18.56,18.56,0,0,0-6.23,4.1A19.05,19.05,0,0,0,80,131.13a19.83,19.83,0,0,0,1.47,7.71,18.71,18.71,0,0,0,10.37,10.22A20.69,20.69,0,0,0,99.65,150.5Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M137.37,156.33v-50.4H157a27.67,27.67,0,0,1,11.3,2.09,16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.38,16.38,0,0,1-7.2,6,27.33,27.33,0,0,1-11.3,2.12h-15.7l3.24-3.38v18.64Zm7.2-17.92-3.24-3.6h15.48q6.91,0,10.48-3a11.57,11.57,0,0,0,0-16.7q-3.57-3-10.48-3H141.33l3.24-3.6Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M195.84,150.07h29.37v6.26H188.64v-50.4H224.2v6.27H195.84Zm-.65-22.39h25.92v6.12H195.19Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M236.88,156.33v-50.4h5.9l33.34,41.4H273v-41.4h7.2v50.4h-5.9L241,114.93h3.1v41.4Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M302.54,156.33h-7.2v-50.4h35.57v6.27H302.54Zm-.72-26.56h26V136h-26Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M341.06,156.33v-50.4h19.65A27.69,27.69,0,0,1,372,108a16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.11,16.11,0,0,1-7.2,6A27.69,27.69,0,0,1,360.71,141H345l3.24-3.31v18.64Zm7.2-17.92L345,134.88H360.5q6.92,0,10.47-3a10.34,10.34,0,0,0,3.57-8.39,10.23,10.23,0,0,0-3.57-8.35q-3.56-3-10.47-3H345l3.24-3.6Zm26.49,17.92-12.81-18.28h7.7l13,18.28Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M393.4,156.33v-50.4h19.66a27.67,27.67,0,0,1,11.3,2.09,16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.38,16.38,0,0,1-7.2,6,27.33,27.33,0,0,1-11.3,2.12h-15.7l3.24-3.38v18.64Zm7.2-17.92-3.24-3.6h15.48q6.92,0,10.48-3a11.57,11.57,0,0,0,0-16.7q-3.57-3-10.48-3H397.36l3.24-3.6Z"
              transform="translate(-72.79 -105.36)" />
          </svg>
        </div>
        Cross Platform Launcher Beta 0.3.1
      </div>
    </div>
    <div class="header-right" data-tauri-drag-region>

      <div class="window-controls">
        <n-button quaternary size="small" @click="minimizeWindow">
          <template #icon>
            <n-icon><Remove /></n-icon>
          </template>
        </n-button>
        <n-button quaternary size="small" @click="maximizeWindow">
          <template #icon>
            <n-icon v-if="isMaximized">
              <Contract />
            </n-icon>
            <n-icon v-else>
              <Expand />
            </n-icon>
          </template>
        </n-button>
        <n-button quaternary size="small" @click="closeWindow" class="close-button">
          <template #icon>
            <n-icon><Close /></n-icon>
          </template>
        </n-button>
      </div>
    </div>
  </div>
</template>
<style scoped>
.header {
  height: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 5px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-logo span {
  font-size: 18px;
  font-weight: bold;
  line-height: 28px;
}

.header-logo svg {
  width: 100px;
  height: auto;
}

.header-logo svg path .actual-light {
  color: rgb(0, 0, 0);
}

.header-logo svg path .actual-dark {
  color: rgb(255, 255, 255);
}

@media screen and (max-width: 280px) {
  .header-logo {
    display: none;
  }
}

.header-right {
  float: right;
  padding-right: 10px;
}


.header-userpanel {
  display: inline-block;
  padding: 16px 16px 16px 0;
}

.header-userpanel .header-userpanel-icon {
  vertical-align: middle;
}

.header-userpanel .header-userpanel-username {
  margin-left: 3px;
  font-size: 12px;
  vertical-align: middle;
}

.header-logout {
  display: inline-block;
  padding: 16px 16px 16px 0;
}

.header-logout .n-button {
  padding-left: 1px;
}

.header-avatar {
  padding: 15px 18px 10px 0;
}


.window-controls {
  display: inline-flex;
  gap: 4px;
  margin-left: 16px;
  align-items: center;
}

.window-controls .n-button {
  width: 32px;
  height: 32px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-button:hover {
  background-color: #e81123 !important;
  color: white !important;
}

[data-tauri-drag-region] {
  cursor: move;
  -webkit-app-region: drag;
  user-select: none;
}

[data-tauri-drag-region="false"] {
  -webkit-app-region: no-drag;
}

.window-controls .n-button {
  -webkit-app-region: no-drag;
}

.header-logo {
  -webkit-app-region: drag;
}
</style>