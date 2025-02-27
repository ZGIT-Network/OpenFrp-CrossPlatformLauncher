<script setup lang="ts">
import { ref, computed } from 'vue';
import { h } from 'vue';
import { RouterLink } from 'vue-router';
import { NMenu,NMenuItem, NIcon, NLayout, NLayoutHeader, NLayoutContent, NText, NLayoutSider } from 'naive-ui';
import type { MenuOption } from 'naive-ui';
import { SettingsOutline, TerminalOutline, HomeOutline, BuildOutline, EnterOutline, AddOutline, InformationOutline } from '@vicons/ionicons5';

const props = defineProps({
  collapsed: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['update:collapsed']);

const toggleCollapse = (value: boolean) => {
  emit('update:collapsed', value);
};

// 菜单配置
const menuOptions: MenuOption[] = [
  {
    label: () => h(RouterLink, { to: { name: 'Home' } }, { default: () => '主页' }),
    key: 'home',
    icon: () => h(HomeOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'CreateProxy' } }, { default: () => '新建隧道' }),
    key: 'newproxy',
    icon: () => h(AddOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'ProxyList' } }, { default: () => '隧道管理' }),
    key: 'proxylist',
    icon: () => h(BuildOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'FrpcManager' } }, { default: () => '日志' }),
    key: 'frpc',
    icon: () => h(TerminalOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'Settings' } }, { default: () => '设置' }),
    key: 'settings',
    icon: () => h(SettingsOutline)
  },
  {
    label: () => h(RouterLink, { to: { name: 'Info' } }, { default: () => '关于' }),
    key: 'info',
    icon: () => h(InformationOutline)
  },
  {
    label: () => h('a', { href: 'https://console.openfrp.net' }, { default: () => '打开网页面板' }),
    key: 'webpanel',
    icon: () => h(EnterOutline)
  }
];

// 设置默认选中的菜单项
const selectedKey = ref('home');
</script>

<template>
  <n-layout-sider 
    bordered 
    collapse-mode="width" 
    :collapsed-width="64" 
    :width="240" 
    :collapsed="collapsed"
    show-trigger 
    @collapse="toggleCollapse(true)" 
    @expand="toggleCollapse(false)"
  >
    <n-menu 
      :collapsed="collapsed" 
      :collapsed-width="64" 
      :collapsed-icon-size="22" 
      :options="menuOptions"
      v-model:value="selectedKey" 
    />
  </n-layout-sider>
</template>

<style scoped>
.n-menu {
  height: 100%;
}
</style>