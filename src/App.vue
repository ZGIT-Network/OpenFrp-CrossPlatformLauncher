<script setup lang="ts">
import { computed, provide, ref, watch, onMounted, onUnmounted } from 'vue';
import { lightTheme ,darkTheme, dateZhCN, useOsTheme, zhCN } from 'naive-ui';
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

const clearAuthAndRedirect = () => {
  sessionStorage.setItem('redirectPath', route.fullPath);
  Cookies.remove('authorization');
  localStorage.removeItem('userToken');
  // 避免重复跳转
  if (route.path !== '/settings') {
    router.push('/settings');
  }
};

// 获取用户信息函数
const getUserInfo = () => {
  frpApiGetUserInfo()
    .then((res) => {
      console.log('App.vue 获取用户信息:', res);
      if (res.flag) {
        userInfo.value = res.data;
        console.log('用户信息已更新:', userInfo.value);
      } else {
        clearAuthAndRedirect();
      }
    })
    .catch((error) => {
      console.error('获取用户信息失败:', error);
      clearAuthAndRedirect();
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

// 背景图片相关
// 背景图片、模糊与覆盖色控制
const backgroundImage = ref<string>(localStorage.getItem('backgroundImage') || '');
const backgroundBlur = ref<number>(parseInt(localStorage.getItem('backgroundBlur') || '40')); // 百分比 20-100
const backgroundOpacity = ref<number>(parseInt(localStorage.getItem('backgroundOpacity') || '100')); // 0-100
const setBackgroundImage = (url: string) => {
  backgroundImage.value = url;
  localStorage.setItem('backgroundImage', url);
};

const applyBackgroundImage = () => {
  if (backgroundImage.value) {
    // 在 body 前插入单独层，避免影响布局
    let layer = document.getElementById('background-image-layer');
    if (!layer) {
      layer = document.createElement('div');
      layer.id = 'background-image-layer';
      Object.assign(layer.style, {
        position: 'fixed',
        top: '0',
        left: '0',
        right: '0',
        bottom: '0',
        zIndex: '-1',
        backgroundSize: 'cover',
        backgroundPosition: 'center center',
        backgroundRepeat: 'no-repeat'
      });
      document.body.prepend(layer);
    }
    layer.style.backgroundImage = `url('${backgroundImage.value}')`;
    document.body.style.backgroundSize = 'cover';
    document.body.style.backgroundPosition = 'center center';
    document.body.style.backgroundRepeat = 'no-repeat';
  } else {
    const layer = document.getElementById('background-image-layer');
    if (layer) layer.remove();
  }
};

// 同步背景
onMounted(applyBackgroundImage);
watch(backgroundImage, applyBackgroundImage);

const setBackgroundBlur = (percent: number) => {
  const val = Math.max(20, Math.min(100, percent));
  backgroundBlur.value = val;
  localStorage.setItem('backgroundBlur', String(val));
};

const applyBackgroundBlur = () => {
  const elem = document.getElementById('background-image-layer');
  if (elem) {
    const px = (backgroundBlur.value / 5).toFixed(1); // 20%->4px,100%->20px
    elem.style.filter = `blur(${px}px)`;
  }
};

watch(backgroundBlur, applyBackgroundBlur);
onMounted(applyBackgroundBlur);

const setBackgroundOpacity = (percent: number) => {
  const val = Math.max(0, Math.min(100, percent));
  backgroundOpacity.value = val;
  localStorage.setItem('backgroundOpacity', String(val));
};

const applyBackgroundOpacity = () => {
  const layer = document.getElementById('background-image-layer');
  if (layer) {
    layer.style.opacity = (backgroundOpacity.value / 100).toString();
  }
};

watch(backgroundOpacity, applyBackgroundOpacity);
onMounted(applyBackgroundOpacity);

// 覆盖色开关
const overlayColorEnabled = ref(localStorage.getItem('overlayColorEnabled') !== 'false');
const toggleOverlayColor = (val: boolean) => {
  overlayColorEnabled.value = val;
  localStorage.setItem('overlayColorEnabled', val ? 'true' : 'false');
};
const applyOverlayColor = () => {
  let ol = document.getElementById('overlay-color-layer');
  if (!overlayColorEnabled.value) {
    if (ol) ol.remove();
    // 关闭时清掉 inline style，恢复由 CSS 接管
    document
      .querySelectorAll<HTMLElement>('.n-layout-header, .n-layout-sider, .n-layout-sider__content')
      .forEach((el) => {
        el.style.backgroundColor = '';
      });
    document.body.classList.remove('overlay-color-enabled');
    return;
  }
  if (!ol) {
    ol = document.createElement('div');
    ol.id = 'overlay-color-layer';
    Object.assign(ol.style, {
      position: 'fixed',
      top: '0', left: '0', right: '0', bottom: '0',
      pointerEvents: 'none',
      zIndex: '-1'
    });
    document.body.prepend(ol);
  }
  const color = colorScheme.value === 'dark' ? 'rgba(30,30,30,0.30)' : 'rgba(255,255,255,0.40)';
  ol.style.backgroundColor = color;

  // 给 Header/Sidebar 统一一个标记 class，便于全局样式兜底
  document.body.classList.add('overlay-color-enabled');
  // 同步 Header 与 Sidebar 背景（包含 sider 内部滚动容器等）
  document
    .querySelectorAll<HTMLElement>('.n-layout-header, .n-layout-sider, .n-layout-sider__content')
    .forEach((el) => {
      el.style.backgroundColor = color;
    });
};

watch([overlayColorEnabled, colorScheme], applyOverlayColor, { immediate: true });

watch(overlayColorEnabled, (val) => {
  document.body.classList.toggle('overlay-color-enabled', val);
}, { immediate: true });

provide('backgroundImageControl', { backgroundImage, setBackgroundImage, backgroundBlur, setBackgroundBlur, backgroundOpacity, setBackgroundOpacity, overlayColorEnabled, toggleOverlayColor });

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
const theme = computed(() => (colorScheme.value === 'dark' ? darkTheme : lightTheme));

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

const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const commonOverrides = {
    primaryColor: '#72a0c9',
    primaryColorHover: '#529bdb',
    primaryColorPressed: '#0F6FB8',
    primaryColorSuppl: '#529bdb',
    borderRadius: '5px',
  };

  if (colorScheme.value === 'dark') {
    // Dark theme overrides
    return {
      common: commonOverrides,
      Layout: {
        headerBorderColor: 'rgba(255, 255, 255, 0.25)',
        siderBorderColor: 'rgba(255, 255, 255, 0.25)',
      },
      Menu: {
        itemColorHover: 'rgba(255, 255, 255, 0.15)',
      },
    };
  } else {
    // Light theme overrides
    return {
      common: commonOverrides,
      Layout: {
        headerBorderColor: 'rgba(0, 0, 0, 0.18)',
        siderBorderColor: 'rgba(0, 0, 0, 0.18)',
      },
      Menu: {
        itemColorHover: 'rgba(0, 0, 0, 0.09)',
      },
    };
  }
});

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

const version = `OFCPL 开发模式 | 开发中功能，不代表最终效果`;
const shouldWatermark = !__DEV_MODE__;
</script>

<template>
  <n-watermark
    v-if="shouldWatermark"
    :content="version"
    cross
    fullscreen
    :font-size="16"
    :line-height="80"
    :width="600"
    :height="800"
    :x-offset="12"
    :y-offset="60"
    :rotate="-12"
    style="z-index: 99999"
  />
  <div id="captcha-box"></div>
  <n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN" :theme="theme">
    <n-dialog-provider>
      <n-loading-bar-provider>
        <n-notification-provider>
          <n-message-provider>
            <n-layout style="height: 100vh;">
              <!-- 头部 -->
              <n-layout-header bordered style="height: 48px; padding: 0">
                <Header />
              </n-layout-header>
              
              <!-- 主体布局 -->
              <n-layout has-sider position="absolute" style="top: 48px; bottom: 0">
                <!-- 侧边栏组件 -->
                <Sidebar v-model:collapsed="collapsed" style="/*background-color: transparent !important;*/"/>
                
                <!-- 内容区域 -->
                <n-layout>
                  <n-layout-content content-style="padding: 24px;">
                    <n-text
                      style="position:fixed;display:flex; right:40px;bottom: 40px;z-index:99999;pointer-events: none; user-select: none;opacity: 0.5;">
                      OpenFrp Cross Platform Launcher<br />Beta v{{currentVersion}} 预览体验计划 {{ userInfo?.username }}
                    </n-text>
                    
                      <router-view style="max-height: calc(100vh - 40px - 40px );"></router-view>
                    
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
  background-color: transparent !important;
}

#app {
  height: 100%;
  width: 100%;
  background-color: transparent !important;
}

.gaussian-blur-enabled {
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
}

.n-layout {
  background-color: rgba(255, 255, 255, 0.20) !important; 
  height: 100%;
}

.n-layout-header {
  background-color: rgba(255, 255, 255, 0.45) !important; 
  height: 100%;
}

.n-layout-sider {
  background-color: rgba(255, 255, 255, 0.45) !important; 
  height: 100%;
}

/* 覆盖色启用后，Header/Sidebar 走同一套色值（JS 会写 inline，这里做兜底/一致性） */


/* body.overlay-color-enabled .n-layout-sider__content {
  background-color: rgba(255, 255, 255, 0.40) !important;
}

body.actual-dark.overlay-color-enabled .n-layout-sider__content {
  background-color: rgba(30, 30, 30, 0.30) !important;
} */

.actual-dark .n-layout  {
  background-color: rgba(30, 30, 30, 0.3) !important; 
  height: 100%;
  /* 暗色主题下内容区域背景 */
}


.actual-dark .n-layout-header  {
  background-color: rgba(30, 30, 30, 0.2) !important; 
  height: 100%;
} 

/* .n-layout-content {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  background-color: rgba(255, 255, 255, 0.4) !important; 
  /* 内容区域半透明但不完全透明 */
  /* height:100%;
  flex: auto;
} */ 

/* .actual-dark .n-layout-content {
  background-color: rgba(30, 30, 30, 0.5) !important; 
  height: 100%;
  /* 暗色主题下内容区域背景 */
/* }  */

.n-layout-sider {
  background-color: transparent;
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

.n-card {
  background-color: rgba(255, 255, 255, 0.4) !important;
  backdrop-filter: blur(8px);
}

.actual-dark .n-card {
  background-color: rgba(30, 30, 30, 0.3) !important;
}

/* 亮色模式样式 */
/* :not(.actual-dark) {
  color-scheme: light;
} */

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
  background-color: transparent !important;
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