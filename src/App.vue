<script setup lang="ts">
import { computed, provide, ref, watch, onMounted } from 'vue';
import { NConfigProvider, NLoadingBarProvider, NNotificationProvider, NMessageProvider, NDialogProvider, NGlobalStyle, NLayout, NLayoutSider, NLayoutHeader, NLayoutContent, NMenu } from 'naive-ui'
import { darkTheme, dateZhCN, useOsTheme, zhCN } from 'naive-ui';
import type { MenuOption } from 'naive-ui'
import { h } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import { SettingsOutline, TerminalOutline, HomeOutline, BuildOutline, EnterOutline, AddOutline, InformationOutline } from '@vicons/ionicons5'
import { GlobalThemeOverrides } from 'naive-ui';


const osThemeRef = useOsTheme();
const colorScheme = ref<'dark' | 'light'>(osThemeRef.value === 'dark' ? 'dark' : 'light');
const collapsed = ref(false);
provide('collapsed', collapsed);

import Header from './layouts/Header/index.vue';

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
        'a',
        {
          href: 
           'https://console.openfrp.net',
        
        },
        { default: () => '打开网页面板' }
      ),
    key: 'webpanel',
    icon: () => h(EnterOutline)
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

// 处理所有外部链接
onMounted(() => {
  document.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    const link = target.closest('a')
    
    if (link && link.href && !link.href.startsWith('javascript:') && !link.href.startsWith('#')) {
      // 检查是否是外部链接
      if (!link.href.startsWith(window.location.origin)) {
        e.preventDefault()
        window.open(link.href, '_blank',"width=1000,height=700,top=100,left=100,resizable=yes,scrollbars=yes")
      }
    }
  })
})
</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN" :theme="theme">
    <n-loading-bar-provider>
      <n-notification-provider>
        <n-message-provider>
          <n-dialog-provider>
            <n-layout style="height: 100vh">
              <!-- 这里恢复了 Header 组件 -->
              <n-layout-header bordered style="height: 64px; padding: 0">
                <Header />
              </n-layout-header>
              <n-layout has-sider position="absolute" style="top: 64px; bottom: 0">
                <n-layout-sider bordered collapse-mode="width" :collapsed-width="64" :width="240" :collapsed="collapsed"
                  show-trigger @collapse="collapsed = true" @expand="collapsed = false">
                    <!-- 这里将 v-model:value 绑定到 selectedKey 上，默认选中 home -->
                    <n-menu :collapsed="collapsed" :collapsed-width="64" :collapsed-icon-size="22"
                      :options="menuOptions" v-model:value="selectedKey" />
                  </n-layout-sider>
                  <n-layout>
                    <n-layout-content content-style="padding: 24px;">
                      <router-view></router-view>
                    </n-layout-content>
                  </n-layout>
              </n-layout>
            </n-layout>
          </n-dialog-provider>
        </n-message-provider>
      </n-notification-provider>
    </n-loading-bar-provider>
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
