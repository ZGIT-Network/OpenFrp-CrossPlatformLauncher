<script setup lang="ts">
import { computed, provide, ref, watch, onMounted, onUnmounted } from 'vue';
import { darkTheme, dateZhCN, useOsTheme, zhCN } from 'naive-ui';
import { GlobalThemeOverrides } from 'naive-ui';
import { NConfigProvider, NLoadingBarProvider, NDialogProvider, NNotificationProvider, NMessageProvider, NGlobalStyle, NLayout, NLayoutHeader, NLayoutContent, NText, NScrollbar } from 'naive-ui';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useRouter, useRoute } from 'vue-router';
// import { setWindowBlurEffect } from '@/utils/windowEffect';

// 导入布局组件
import Header from './layouts/Header/index.vue';
import Sidebar from './layouts/Sidebar/index.vue';
import frpApiGetUserInfo from '@/requests/frpApi/frpApiGetUserInfo';
import Cookies from '@/utils/cookies';

// 添加用户信息相关代码
const userInfo = ref<Struct.UserInfo>();
const router = useRouter();
const route = useRoute();

// 获取用户信息函数
const getUserInfo = () => {
  frpApiGetUserInfo()
    .then((res) => {
      console.log('App.vue 获取用户信息:', res);
      if (res.flag) {
        userInfo.value = res.data;
        console.log('用户信息已更新:', userInfo.value);
      } else {
        // 需要登录的情况
        sessionStorage.setItem('redirectPath', route.fullPath);
        Cookies.remove('authorization');
        router.push('/settings');
      }
    })
    .catch((error) => {
      console.error('获取用户信息失败:', error);
    });
};

// 在应用启动时获取用户信息
getUserInfo();

// 提供用户信息给所有子组件
provide('userInfo', { userInfo, getUserInfo });

// 其他现有代码保持不变
const osThemeRef = useOsTheme();
const colorScheme = ref<'dark' | 'light'>(osThemeRef.value === 'dark' ? 'dark' : 'light');
const collapsed = ref(false);
provide('collapsed', collapsed);

const currentVersion = ref('v0.1');
const getCurrentVersion = async (retries = 3) => {
  for (let i = 0; i < retries; i++) {
    try {
      const version = await invoke('get_cpl_version');
      currentVersion.value = version as string;
      return;
    } catch (e) {
      console.error(`获取版本失败 (尝试 ${i + 1}/${retries}):`, e);
      if (i === retries - 1) {
        currentVersion.value = '获取失败';
      } else {
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
    }
  }
};
getCurrentVersion();

// 主题相关
watch(osThemeRef, () => beAttachToggleColorScheme(osThemeRef.value === 'dark' ? 'dark' : 'light'));
const theme = computed(() => (colorScheme.value === 'dark' ? darkTheme : null));

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
const cleanupFunctions = ref<(() => void)[]>([]);

// 添加日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  const logMessage = `[${timestamp}] [系统] ${message}\n`;
  const savedLogs = localStorage.getItem('frpcLogs') || '';
  localStorage.setItem('frpcLogs', savedLogs + logMessage);
};

// 添加一个标志来记录日志系统是否已经初始化
const isLogSystemInitialized = ref(false);

onMounted(async () => {
  // 如果日志系统已经初始化，直接返回
  if (isLogSystemInitialized.value) {
    return;
  }

  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message);
    });
    cleanupFunctions.value.push(globalLogUnlisten);
    // 其他监听器...
    isLogSystemInitialized.value = true;
  } catch (e) {
    console.error('初始化日志系统失败:', e);
  }

 
});

onUnmounted(() => {
  // 清理所有监听器
  cleanupFunctions.value.forEach((cleanup: any) => cleanup());
});

// 禁止右键菜单
document.oncontextmenu = function (event: any) {
  if (window.event) {
    event = window.event;
  }
  try {
    var the = event.srcElement;
    if (!(
      (the.tagName == 'INPUT' && the.type.toLowerCase() == 'text') ||
      the.tagName == 'TEXTAREA'
    )) {
      return false;
    }
    return true;
  } catch (e) {
    return false;
  }
};
</script>

<template>
  <div id="captcha-box"></div>
  <n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN" :theme="theme">
    <n-dialog-provider>
      <n-loading-bar-provider>
        <n-notification-provider>
          <n-message-provider>
            <n-layout style="height: 100vh">
              <!-- 头部 -->
              <n-layout-header bordered style="height: 55px; padding: 0">
                <Header />
              </n-layout-header>
              
              <!-- 主体布局 -->
              <n-layout has-sider position="absolute" style="top: 55px; bottom: 0">
                <!-- 侧边栏组件 -->
                <Sidebar v-model:collapsed="collapsed" />
                
                <!-- 内容区域 -->
                <n-layout>
                  <n-layout-content content-style="padding: 24px;">
                    <n-text
                      style="position:fixed;display:flex; right:40px;bottom: 40px;z-index:99999;pointer-events: none; user-select: none;opacity: 0.5;">
                      OpenFrp Cross Platform Launcher<br />Beta v{{currentVersion}} 预览体验计划 {{ userInfo?.username }}
                    </n-text>
                    <n-scrollbar style="max-height: calc(100vh - 64px - 40px);">
                      <router-view></router-view>
                    </n-scrollbar>
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

a {
  text-decoration: none;
  color: #72a0c9;
}

a:hover {
  color: #529bdb;
}

.animate__animated {
  --animate-duration: 0.55s;
}

/* 暗色模式样式 */
.actual-dark {
  color-scheme: dark light;
}

/* 亮色模式样式 */
:not(.actual-dark) {
  color-scheme: light;
}

/* 根样式 */
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

/* 处理亮/暗模式样式 */
.actual-dark {
  color-scheme: dark light;
}

.actual-light {
  color-scheme: light;
}

/* 高斯模糊视觉特效，兼容 Naive UI 深色/亮色模式 */
body,
#app,
.n-layout,
.n-layout-content,
.n-layout-sider,
.n-layout-header {
  background: transparent !important;
}

body.blur-enabled .n-layout,
body.blur-enabled .n-layout-content,
body.blur-enabled .n-layout-sider,
body.blur-enabled .n-layout-header {
  /* 亮色高斯模糊 */
  background: var(--n-color) !important;

  filter: blur(0px); /* 兼容性兜底 */
  backdrop-filter: blur(24px) saturate(1.2) brightness(1.08);
  -webkit-backdrop-filter: blur(24px) saturate(1.2) brightness(1.08);
  transition: background 0.3s;
}
body.actual-dark.blur-enabled .n-layout,
body.actual-dark.blur-enabled .n-layout-content,
body.actual-dark.blur-enabled .n-layout-sider,
body.actual-dark.blur-enabled .n-layout-header {
  /* 深色高斯模糊 */
  background: var(--n-color) !important;

  filter: blur(0px);
  backdrop-filter: blur(24px) saturate(1.2) brightness(0.85);
  -webkit-backdrop-filter: blur(24px) saturate(1.2) brightness(0.85);
}
body:not(.blur-enabled) .n-layout,
body:not(.blur-enabled) .n-layout-content,
body:not(.blur-enabled) .n-layout-sider,
body:not(.blur-enabled) .n-layout-header {
  background: unset !important;
  filter: none !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

/* 高斯模糊特效（全局，兼容深色/NaiveUI） */
body.gaussian-blur-enabled .n-layout,
body.gaussian-blur-enabled .n-layout-content,
body.gaussian-blur-enabled .n-layout-sider,
body.gaussian-blur-enabled .n-layout-header {
  background: var(--n-color) !important;

  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  transition: background 0.3s;
}
body.actual-dark.gaussian-blur-enabled .n-layout,
body.actual-dark.gaussian-blur-enabled .n-layout-content,
body.actual-dark.gaussian-blur-enabled .n-layout-sider,
body.actual-dark.gaussian-blur-enabled .n-layout-header {
  background: var(--n-color) !important;

  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}
body:not(.gaussian-blur-enabled) .n-layout,
body:not(.gaussian-blur-enabled) .n-layout-content,
body:not(.gaussian-blur-enabled) .n-layout-sider,
body:not(.gaussian-blur-enabled) .n-layout-header {
  background: var(--n-color) !important;

}
</style>