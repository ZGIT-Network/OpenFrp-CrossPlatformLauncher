<script lang="ts" setup>
import { ref, computed, inject, h, onMounted, onUnmounted, watch } from 'vue';
import type { Ref } from 'vue';
import { NButton, useDialog, useMessage, useNotification } from 'naive-ui';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { RouterLink, useRouter } from 'vue-router';
import { Remove, Expand, Contract, Close, Refresh, Moon, Sunny } from '@vicons/ionicons5';
import copy from 'copy-to-clipboard';
import frpApiGetUserProxies from '@/requests/frpApi/frpApiGetUserProxies';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';
import { useAppUpdates } from '@/services/appUpdates';
import type { UpdateInfo } from '@/services/appUpdates';
import { globalLogService } from '@/services/logService';
import { notifySystem, useTunnelStartedNotice } from '@/services/tunnelNotifications';
import { getSavedTunnelIds, pendingTunnels, startManagedTunnel } from '@/services/tunnelRuntime';
import { proxyRemoteAddress } from '@/utils/proxies';

const router = useRouter();
const message = useMessage();
const notification = useNotification();
const showTunnelStarted = useTunnelStartedNotice();
const dialog = useDialog();
const session = useSessionStore();
const appWindow = getCurrentWindow();
const { installing, installUpdate } = useAppUpdates();

const isMaximized = ref(false);
const minimizeWindow = () => appWindow.minimize().catch(console.error);
const closeWindow = () => appWindow.close().catch(console.error);
async function maximizeWindow() {
  try {
    if (await appWindow.isMaximized()) await appWindow.unmaximize();
    else await appWindow.maximize();
    isMaximized.value = await appWindow.isMaximized();
  } catch (error) {
    console.error('调整窗口失败:', error);
  }
}
function refreshWebview() {
  globalLogService.flush();
  window.location.reload();
}
function copyAddress(address: string) {
  const first = address.split('\n').find(line => line.trim())?.trim();
  if (!first || !copy(first)) message.error('复制链接地址失败');
  else message.success('链接地址已复制到剪贴板');
}

const restoreAtStartup = ref(false);
watch([restoreAtStartup, () => session.userInfo], async ([restore, account]) => {
  if (!restore || !account?.token) return;
  restoreAtStartup.value = false;
  const ids = getSavedTunnelIds(account.id);
  if (!ids.length) return;
  try {
    if (!await invoke<boolean>('check_frpc')) return;
    const response = await frpApiGetUserProxies();
    if (!response.flag) throw new Error(response.msg);
    if (session.userInfo?.id !== account.id) return;

    const selected = response.data.list.filter(proxy =>
      ids.includes(String(proxy.id)) && proxy.status === true && proxy.online === false
    );
    if (!selected.length) return;
    await router.push('/proxylist');
    message.info('正在恢复上次运行的隧道');
    for (const proxy of selected) {
      if (session.userInfo?.id !== account.id) break;
      try {
        await startManagedTunnel({
          id: String(proxy.id), token: session.userInfo.token, name: proxy.proxyName,
          remote: proxyRemoteAddress(proxy), external: false,
        });
      } catch (error) {
        message.error(`恢复隧道 #${proxy.id} 失败：${errorMessage(error)}`);
      }
    }
  } catch (error) {
    message.error(`恢复隧道失败：${errorMessage(error)}`);
  }
});

const handledLoginCodes = new Set<string>();
async function handleDeepLink(value: string) {
  let quickStartLabel: string | undefined;
  try {
    const url = new URL(value);
    if (url.protocol !== 'openfrp:') return;
    if (url.hostname === 'login' || url.hostname === 'oauth_callback') {
      const code = url.searchParams.get('code');
      if (!code) {
        message.error('登录链接缺少授权码');
        return;
      }
      if (session.signedIn || handledLoginCodes.has(code)) return;
      handledLoginCodes.add(code);
      if (handledLoginCodes.size > 32) handledLoginCodes.delete(handledLoginCodes.values().next().value!);
      await router.push({ path: '/oauth_callback', query: { code } });
      return;
    }
    if (url.hostname !== 'start_proxy') {
      message.error('未知链接操作');
      return;
    }
    const id = (url.searchParams.get('proxy') || '').replace(/^0+(?=\d)/, '');
    const token = url.searchParams.get('user')?.trim() || '';
    const remote = url.searchParams.get('remote')?.trim() || '';
    const name = url.searchParams.get('name')?.trim() || '';
    if (!/^[1-9]\d{0,19}$/.test(id) || !token || !remote || !name) {
      message.error('快速启动链接格式错误');
      void notifySystem('快速启动链接格式错误', '请检查链接格式是否正确');
      return;
    }
    if (pendingTunnels.has(id)) return;
    quickStartLabel = `隧道 #${id} ${name}`;
    if (!await invoke<boolean>('get_quick_start_enabled')) return;
    if (pendingTunnels.has(id)) return;
    message.info(`正在快速启动${name}隧道`);
    const result = await startManagedTunnel({ id, token, name, remote, external: true });
    if (result === 'already-running') {
      message.info(`隧道 #${id} 已在运行`);
    } else if (result === 'running') {
      message.warning(`隧道 #${id} 的进程已启动，连接结果请查看日志`);
      void notifySystem(`${quickStartLabel} 的进程已启动`, '连接结果请查看日志');
    } else {
      showTunnelStarted(`${quickStartLabel} 启动成功`, remote, () => copyAddress(remote));
    }
  } catch (error) {
    const detail = errorMessage(error);
    message.error(`处理链接失败：${detail}`);
    if (quickStartLabel) void notifySystem(`${quickStartLabel} 启动失败`, detail);
  }
}

async function checkForUpdates() {
  try {
    const update = await invoke<UpdateInfo | null>('check_update');
    if (!update) return;
    const notice = notification.info({
      title: update.title,
      description: `发现新版本 v${update.latest}\n${update.msg}`,
      duration: 10000,
      content: () => h(NButton, {
        text: true, type: 'primary', disabled: installing.value,
        onClick: () => { notice.destroy(); void installUpdate(update.latest); },
      }, () => '立即更新'),
    });
  } catch (error) {
    console.error('检查更新失败:', error);
  }
}

let disposed = false;
let resizeTimer: ReturnType<typeof setTimeout> | undefined;
let closeDialog: ReturnType<typeof dialog.warning> | undefined;
const cleanups: (() => void)[] = [];
async function register(listener: Promise<() => void>) {
  try {
    const cleanup = await listener;
    if (disposed) cleanup();
    else cleanups.push(cleanup);
  } catch (error) {
    console.error('注册窗口监听失败:', error);
  }
}

function handleClose() {
  if (closeDialog) return;
  closeDialog = dialog.warning({
    title: '二次确认',
    content: '确认退出程序？',
    positiveText: '最小化到托盘',
    negativeText: '彻底退出',
    onAfterLeave: () => { closeDialog = undefined; },
    onPositiveClick: () => appWindow.hide().catch(error => message.error(errorMessage(error))),
    onNegativeClick: async () => {
      try {
        globalLogService.flush();
        await invoke('exit_app');
      } catch (error) {
        message.error(`退出失败：${errorMessage(error)}`);
        return false;
      }
    },
  });
}

onMounted(() => {
  void appWindow.isMaximized().then(value => { isMaximized.value = value; }).catch(console.error);
  // 缩放时减少 Windows 和 macOS 发往主线程的 IPC。
  void register(appWindow.onResized(() => {
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      void appWindow.isMaximized().then(value => { isMaximized.value = value; }).catch(console.error);
    }, 150);
  }));
  void register(appWindow.onCloseRequested(event => {
    event.preventDefault();
    handleClose();
  }));
  void register(listen<string>('tray-copy-link', ({ payload }) => copyAddress(payload)));
  void register(listen<string>('tray-error', ({ payload }) => message.error(payload)));
  void register(onOpenUrl(urls => urls.forEach(url => void handleDeepLink(url))))
    .then(() => invoke<string[]>('take_startup_links'))
    .then(urls => urls.forEach(url => void handleDeepLink(url))).catch(console.error);

  void invoke<boolean>('take_autostart_restore').then(value => {
    restoreAtStartup.value = value && session.signedIn && localStorage.getItem('autoRestoreTunnels') !== 'false';
  }).catch(console.error);
  void invoke<boolean>('check_frpc').then(installed => {
    if (installed) return;
    const notice = notification.warning({
      title: '未检测到 Frpc',
      description: '请下载或手动配置 Frpc 后启动隧道。',
      duration: 8000,
      content: () => h(RouterLink, {
        to: { path: '/settings', query: { section: 'frpc' } },
        onClick: () => notice.destroy(),
      }, () => '前往配置'),
    });
  }).catch(console.error);
  void checkForUpdates();
});

onUnmounted(() => {
  disposed = true;
  if (resizeTimer) clearTimeout(resizeTimer);
  cleanups.forEach(cleanup => cleanup());
});

const { colorScheme, toggleColorScheme: toggleDarkMode } = inject('darkMode') as {
  colorScheme: Ref<string>;
  toggleColorScheme: () => void;
};
const isDark = computed(() => colorScheme.value === 'dark');
</script>

<template>
  <div class="header" data-tauri-drag-region @dblclick="maximizeWindow">
    <div class="header-left" data-tauri-drag-region>
      <div class="header-logo" data-tauri-drag-region>
        <div class="brand-wrap">
          <svg fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 361.29 51.55" aria-label="OpenFrp">
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
          <span class="brand-text">Cross Platform Launcher</span>
        </div>
      </div>
    </div>
    <div class="header-right" data-tauri-drag-region>

      <div class="window-controls" @dblclick.stop>
        <n-tooltip placement="bottom">
          <template #trigger>
            <n-button quaternary circle size="small" @click="toggleDarkMode" :aria-label="isDark ? '切换到浅色主题' : '切换到深色主题'">
              <template #icon>
                <n-icon>
                  <Moon v-if="isDark" />
                  <Sunny v-else />
                </n-icon>
              </template>
            </n-button>
          </template>
          {{ isDark ? '切换到浅色主题' : '切换到深色主题' }}
        </n-tooltip>
        <n-tooltip>
          刷新(遇到 bug 时可尝试)
          
          <template #trigger>
            <n-button quaternary size="small" @click="refreshWebview" aria-label="刷新启动器">
           
              <template #icon>
                <n-icon>
                  <Refresh />
                </n-icon>
              </template>
            </n-button>
          </template>
        </n-tooltip>
        <n-button quaternary size="small" @click="minimizeWindow" aria-label="最小化">
          <template #icon>
            <n-icon>
              <Remove />
            </n-icon>
          </template>
        </n-button>
        <n-button quaternary size="small" @click="maximizeWindow" :aria-label="isMaximized ? '还原窗口' : '最大化'">
          <template #icon>
            <n-icon v-if="isMaximized">
              <Contract />
            </n-icon>
            <n-icon v-else>
              <Expand />
            </n-icon>
          </template>
        </n-button>
        <n-button quaternary size="small" @click="closeWindow" class="close-button" aria-label="关闭">
          <template #icon>
            <n-icon>
              <Close />
            </n-icon>
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
  background-color: rgba(255, 255, 255, 0.01); 
  /* 调整为半透明背景 */
  backdrop-filter: blur(10px);    
  -webkit-backdrop-filter: blur(10px);
}

/* 暗色主题下的透明效果 */
.actual-dark .header {
  background-color: rgba(30, 30, 30, 0.4); 
  /* 暗色主题下的半透明背景 */
}


.header-left {
  margin-left: 1em;
  display: flex;
  align-items: center;
  gap: 16px;
}

.brand-wrap {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.header-logo .brand-text {
  font-size: 17px;
  font-weight: normal;
  color: var(--n-text-color);
  line-height: 1;
  white-space: nowrap;
}

.header-logo svg {
  width: 100px;
  min-width: 100px;
  height: auto;
  color: var(--n-text-color);
  display: block;
  flex-shrink: 0;
}

.header-logo svg path {
  fill: currentColor;
}

@media (max-width: 540px) {
  .header-logo .brand-text { display: none; }
}

.header-right {
  float: right;
  padding-right: 10px;
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
