<script setup lang="ts">
import { ref, watch, inject, computed, Ref } from 'vue';
import { h } from 'vue';
import { RouterLink, useRoute } from 'vue-router';
import { NMenu, NLayoutSider } from 'naive-ui';
import type { MenuOption } from 'naive-ui';
import { SettingsOutline, TerminalOutline, HomeOutline, BuildOutline, LogoWebComponent, EnterOutline, AddOutline, InformationOutline } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import  Coockies  from '@/utils/cookies';

import { openUrl } from '@tauri-apps/plugin-opener';

// 获取当前路由
const route = useRoute();

// 注入用户信息
const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');
const userInfo = userInfoObj?.userInfo;

// 计算用户是否已登录
const isLoggedIn = computed(() => {
  return !!userInfo?.value?.token;
});

const props = defineProps({
  collapsed: {
    type: Boolean,
    default: false
  }
});

// 使用计算属性或直接使用props.collapsed，而不是创建独立的ref
// const collapsed = ref(props.collapsed);

const emit = defineEmits(['update:collapsed']);

const toggleCollapse = (value: boolean) => {
  emit('update:collapsed', value);
};
// 菜单配置
const menuOptions = computed(() => [
  {
    label: () => h(RouterLink, { to: { name: 'Home' } }, () => '主页'),
    key: 'home',
    icon: () => h(HomeOutline)
  },
  {
    label: () => {
      // 根据登录状态决定是否使用RouterLink或普通span
      return isLoggedIn.value
        ? h(RouterLink, { to: { name: 'CreateProxy' } }, () => '新建隧道')
        : h('span', { class: 'disabled-menu-item' }, '新建隧道');
    },
    key: 'newproxy',
    icon: () => h(AddOutline),
    disabled: !isLoggedIn.value
  },
  {
    label: () => h(RouterLink, { to: { name: 'ProxyList' } }, () => '隧道管理'),
    key: 'proxylist',
    icon: () => h(BuildOutline)
  },
  // {
  //   label: () => {
  //     // 根据登录状态决定是否使用RouterLink或普通span
  //     return isLoggedIn.value
  //       ? h(RouterLink, { to: { name: 'NodeStatus' } }, () => '节点状态')
  //       : h('span', { class: 'disabled-menu-item' }, '节点状态');
  //   },
  //   key: 'NodeStatus',
  //   icon: () => h(ServerOutline),
  //   disabled: !isLoggedIn.value
  // },
  {
    label: () => h(RouterLink, { to: { name: 'FrpcManager' } }, () => '日志'),
    key: 'frpc',
    icon: () => h(TerminalOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'Network' } }, () => '网络'),
    key: 'network',
    icon: () => h(LogoWebComponent)
  },
  {
    label: () => h(RouterLink, { to: { name: 'Settings' } }, () => '设置'),
    key: 'settings',
    icon: () => h(SettingsOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'Info' } }, () => '关于'),
    key: 'info',
    icon: () => h(InformationOutline)
  },
  {
    label: '打开网页面板',
    key: 'webpanel',
    icon: () => h(EnterOutline)
  }
]);

// 设置默认选中的菜单项
const selectedKey = ref('home');

// 处理菜单点击事件
const handleMenuSelect = async (key: string) => {
  if (key === 'webpanel') {
    try {
      const auth = Coockies.get('authorization');
      if (auth) {
        await openUrl('https://console.openfrp.net/fastlogin?auth=' + auth);
      } else {
        await openUrl('https://console.openfrp.net/');
      }
    } catch (error) {
      console.error('打开网页面板失败:', error);
    }
  } else {
    selectedKey.value = key;
  }
};

// 监听路由变化，更新选中的菜单项
watch(() => route.name, (newRouteName) => {
  if (newRouteName) {
    // 将路由名称转换为小写，以匹配菜单key
    const routeKey = newRouteName.toString().toLowerCase();

    // 检查是否有匹配的菜单项
    const menuItem = menuOptions.value.find(item => item.key === routeKey);
    if (menuItem) {
      selectedKey.value = routeKey;
    }
  }
}, { immediate: true });
</script>

<template>
  <n-layout-sider 
    bordered 
    collapse-mode="width" 
    :collapsed-width="64" 
    :width="240" 
    :collapsed="props.collapsed"
    show-trigger 
    @collapse="toggleCollapse(true)" 
    @expand="toggleCollapse(false)"
  >
    <n-menu
      :collapsed="props.collapsed"
      :collapsed-width="64"
      :collapsed-icon-size="22"
      :options="menuOptions"
      v-model:value="selectedKey"
      @update:value="handleMenuSelect"
    />
  </n-layout-sider>
</template>

<style scoped>
.n-menu {
  height: 100%;
}

.n-layout-sider {
  background-color: rgba(250, 250, 250, 0.388) !important; 
  /* 增加透明度 */
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
}

/* 暗色主题下的样式 */
.actual-dark .n-layout-sider {
  background-color: rgba(30, 30, 30, 0.4) !important;
}

/* 收起状态下的样式 */
.n-layout-sider--collapsed {
  background-color: rgba(250, 250, 250, 0.2) !important;
}

.actual-dark .n-layout-sider--collapsed {
  background-color: rgba(30, 30, 30, 0.2) !important;
}
</style>