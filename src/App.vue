<script setup lang="ts">
import { computed, provide, ref, watch, onMounted, onUnmounted } from 'vue';
import { darkTheme, dateZhCN, useOsTheme, zhCN } from 'naive-ui';
import { GlobalThemeOverrides } from 'naive-ui';
import { NConfigProvider, NLoadingBarProvider, NDialogProvider, NNotificationProvider, NMessageProvider, NGlobalStyle, NLayout, NLayoutHeader, NLayoutContent, NText, NScrollbar } from 'naive-ui';

import { invoke } from '@tauri-apps/api/core';
import { useRouter, useRoute } from 'vue-router';
// import { setWindowBlurEffect } from '@/utils/windowEffect';

// 导入布局组件
import Header from './layouts/Header/index.vue';
import Sidebar from './layouts/Sidebar/index.vue';
import frpApiGetUserInfo from '@/requests/frpApi/frpApiGetUserInfo';
import Cookies from '@/utils/cookies';
import { globalLogService } from '@/services/logService';

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

onMounted(async () => {
  console.log('App.vue 挂载，初始化全局日志服务');

  try {
    // 初始化全局日志服务
    const success = await globalLogService.initialize();
    if (success) {
      console.log('全局日志服务初始化成功');
    } else {
      console.error('全局日志服务初始化失败');
    }
  } catch (e) {
    console.error('初始化全局日志服务时出错:', e);
  }
});

onUnmounted(() => {
  console.log('App.vue 卸载，清理全局日志服务');
  // 注意：这里不清理全局日志服务，因为它应该在整个应用生命周期内保持活跃
  // globalLogService.cleanup();
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
</style>