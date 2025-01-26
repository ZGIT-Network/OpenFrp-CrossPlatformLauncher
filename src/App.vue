<script setup lang="ts">
import { computed, provide, ref, watch, onMounted, onUnmounted } from 'vue';
import { darkTheme, dateZhCN, useOsTheme, zhCN } from 'naive-ui';
import type { MenuOption } from 'naive-ui'
import { h } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import { SettingsOutline, TerminalOutline, HomeOutline, BuildOutline, EnterOutline, AddOutline, InformationOutline } from '@vicons/ionicons5'
import { GlobalThemeOverrides } from 'naive-ui';
import { NConfigProvider, NLoadingBarProvider, NDialogProvider, NNotificationProvider, NMessageProvider, NGlobalStyle, NLayout, NLayoutSider, NLayoutHeader, NLayoutContent, NMenu, NText } from 'naive-ui'
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

const osThemeRef = useOsTheme();
const colorScheme = ref<'dark' | 'light'>(osThemeRef.value === 'dark' ? 'dark' : 'light');
const collapsed = ref(false);
provide('collapsed', collapsed);

import Header from './layouts/Header/index.vue';

const currentVersion = ref('v0.1')

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

// 菜单配置

watch(osThemeRef, () => beAttachToggleColorScheme(osThemeRef.value === 'dark' ? 'dark' : 'light'));

const theme = computed(() => (colorScheme.value === 'dark' ? darkTheme : null));

const menuOptions: MenuOption[] = [
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'Home',
          },
        },
        { default: () => '主页' }
      ),
    key: 'home',
    icon: () => h(HomeOutline)
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'CreateProxy',
          },
        },
        { default: () => '新建隧道' }
      ),
    key: 'newproxy',
    icon: () => h(AddOutline)
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'ProxyList',
          },
        },
        { default: () => '隧道管理' }
      ),
    key: 'proxylist',
    icon: () => h(BuildOutline)
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'FrpcManager',
          },
        },
        { default: () => '日志' }
      ),
    key: 'frpc',
    icon: () => h(TerminalOutline)
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'Settings',
          },
        },
        { default: () => '设置' }
      ),
    key: 'settings',
    icon: () => h(SettingsOutline)
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            name: 'Info',
          },
        },
        { default: () => '关于' }
      ),
    key: 'info',
    icon: () => h(InformationOutline)
  },
  {
    label: () =>
      h(
        'a',
        {
          href:
            'https://console.openfrp.net',

        },
        { default: () => '打开网页面板' }
      ),
    key: 'webpanel',
    icon: () => h(EnterOutline)
  }

]

// 设置默认选中的菜单项
const selectedKey = ref('home');

const updateBodyColorSchemeForCssColorScheme = () => {
  if (theme.value !== darkTheme) {
    document.documentElement.classList.remove('actual-dark');
  } else {
    document.documentElement.classList.add('actual-dark');
  }
};
const toggleColorScheme = () => {
  if (colorScheme.value === 'dark') {
    colorScheme.value = 'light';
  } else {
    colorScheme.value = 'dark';
  }
  window.localStorage.setItem('colorScheme', colorScheme.value);
};

const beAttachToggleColorScheme = (x: string) => {
  if (x === 'dark' && colorScheme.value === 'light') {
    colorScheme.value = 'dark';
  } else if (x === 'light' && colorScheme.value === 'dark') {
    colorScheme.value = 'light';
  }
  window.localStorage.setItem('colorScheme', colorScheme.value);
};

provide('darkMode', { colorScheme, toggleColorScheme });
watch(theme, () => updateBodyColorSchemeForCssColorScheme());
updateBodyColorSchemeForCssColorScheme();

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#72a0c9',
    primaryColorHover: '#529bdb',
    primaryColorPressed: '#0F6FB8',
    primaryColorSuppl: '#529bdb',
  },
};

// 存储所有监听器的清理函数
const cleanupFunctions = ref<(() => void)[]>([])

// 添加日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  const logMessage = `[系统] [${timestamp}] ${message}\n`
  const savedLogs = localStorage.getItem('frpcLogs') || ''
  localStorage.setItem('frpcLogs', savedLogs + logMessage)
}

// 添加一个标志来记录日志系统是否已经初始化
const isLogSystemInitialized = ref(false)

onMounted(async () => {
  // 如果日志系统已经初始化，直接返回
  if (isLogSystemInitialized.value) {
    return
  }

  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message)
    })
    cleanupFunctions.value.push(globalLogUnlisten)

    // 监听隧道事件
    const tunnelEventUnlisten = await listen('tunnel-event', (event: any) => {
      const { type, tunnelId, tunnelName } = event.payload
      const timestamp = new Date().toLocaleTimeString()
      let message = ''

      switch (type) {
        case 'start':
          message = `[系统] [${timestamp}] 开始启动隧道 ${tunnelName} (ID: ${tunnelId})`
          break
        case 'stop':
          message = `[系统] [${timestamp}] 停止隧道 ${tunnelName} (ID: ${tunnelId})`
          break
        case 'success':
          message = `[系统] [${timestamp}] 隧道 ${tunnelName} (ID: ${tunnelId}) 启动成功`
          break
        case 'error':
          message = `[系统] [${timestamp}] 隧道 ${tunnelName} (ID: ${tunnelId}) 发生错误`
          break
      }

      if (message) {
        const savedLogs = localStorage.getItem('frpcLogs') || ''
        localStorage.setItem('frpcLogs', savedLogs + message + '\n')
      }
    })
    cleanupFunctions.value.push(tunnelEventUnlisten)

    // 为所有可能的隧道ID设置监听器
    const savedStates = localStorage.getItem('tunnelStates')
    if (savedStates) {
      const states = JSON.parse(savedStates)
      for (const id of Object.keys(states)) {
        const instanceLogUnlisten = await listen(`frpc-log-${id}`, (event: any) => {
          const timestamp = new Date().toLocaleTimeString()
          const logMessage = `[隧道 ${id}] [${timestamp}] ${event.payload.message}\n`
          const savedLogs = localStorage.getItem('frpcLogs') || ''
          localStorage.setItem('frpcLogs', savedLogs + logMessage)
        })
        cleanupFunctions.value.push(instanceLogUnlisten)
      }
    }

    // 标记日志系统已初始化
    isLogSystemInitialized.value = true
    appendLog('日志系统启动完成')
  } catch (error) {
    console.error('设置日志监听器时出错:', error)
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
  isLogSystemInitialized.value = false
})

// 处理所有外部链接
onMounted(() => {
  document.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    const link = target.closest('a')

    if (link && link.href && !link.href.startsWith('javascript:') && !link.href.startsWith('#')) {
      // 检查是否是外部链接
      if (!link.href.startsWith(window.location.origin)) {
        e.preventDefault()
        window.open(link.href, '_blank', "width=1000,height=700,top=100,left=100,resizable=yes,scrollbars=yes")
      }
    }
  })


})


</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN" :theme="theme">
    <n-dialog-provider>
      <n-loading-bar-provider>
        <n-notification-provider>
          <n-message-provider>

            <n-layout style="height: 100vh">
              <!-- 这里恢复了 Header 组件 -->
              <n-layout-header bordered style="height: 64px; padding: 0">
                <Header />
              </n-layout-header>
              <n-layout has-sider position="absolute" style="top: 64px; bottom: 0">
                <n-layout-sider bordered collapse-mode="width" :collapsed-width="64" :width="240" :collapsed="collapsed"
                  show-trigger @collapse="collapsed = true" @expand="collapsed = false">
                  <!-- 这里将 v-model:value 绑定到 selectedKey 上，默认选中 home -->
                  <n-menu :collapsed="collapsed" :collapsed-width="64" :collapsed-icon-size="22" :options="menuOptions"
                    v-model:value="selectedKey" />
                </n-layout-sider>
                <n-layout>
                  <n-layout-content content-style="padding: 24px;">
                    <n-text
                      style="position:fixed;display:flex; right:40px;bottom: 40px;z-index:99999;pointer-events: none; user-select: none;opacity: 0.5;">OpenFrp
                      Cross Platform Launcher<br />Tech_Test 技术测试 v{{currentVersion}} 预览体验计划</n-text>
                    <router-view></router-view>
                  </n-layout-content>
                </n-layout>
              </n-layout>
            </n-layout>

          </n-message-provider>
        </n-notification-provider>
      </n-loading-bar-provider>
    </n-dialog-provider>
    <n-global-style />
  </n-config-provider>
</template>

<style>
html,
body {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
}

#app {
  height: 100%;
  width: 100%;
}

.n-layout-content {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  flex: auto;
}

.n-menu {
  height: 100%;
}

a {
  text-decoration: none;
  color: #72a0c9;
}

a:hover {
  color: #529bdb;
}

.animate__animated {
  /* 类名 */
  --animate-duration: 0.55s;
}

/* .actual-dark color-scheme */
.actual-dark {
  /* Dark mode styles */
  color-scheme: dark light;
}

/* :not(.actual-dark) color-scheme */
:not(.actual-dark) {
  color-scheme: light;
}

/* Root styles */
:root {
  font-family: 'Segoe UI', sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol' !important;
  height: 100%;
}

body {
  margin: 0;
  padding: 0;
  border: 0;
}

#root,
.actual-dark,
.actual-light {
  height: 100%;
}

@media screen and (min-width: 700px) {

  .n-config-provider,
  body {
    height: 100%;
  }

  #root {
    height: 100%;
  }
}

/* Handle light/dark mode styles */
.actual-dark {
  /* Dark mode styles */
  color-scheme: dark light;
}

.actual-light {
  color-scheme: light;
}
</style>
