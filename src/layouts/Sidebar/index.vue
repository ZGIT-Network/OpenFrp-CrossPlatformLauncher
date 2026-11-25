<script setup lang="ts">
import { computed, h } from 'vue';
import type { Component } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { NIcon, useMessage } from 'naive-ui';
import {
  SettingsOutline, TerminalOutline, HomeOutline, BuildOutline,
  LogoWebComponent, EnterOutline, AddOutline, InformationOutline,
} from '@vicons/ionicons5';
import { openUrl } from '@tauri-apps/plugin-opener';
import { authorization } from '@/utils/apiClient';
import { useSessionStore } from '@/stores/session';

const route = useRoute();
const router = useRouter();
const session = useSessionStore();
const message = useMessage();
defineProps<{ collapsed: boolean }>();
const emit = defineEmits<{ 'update:collapsed': [value: boolean] }>();

const renderIcon = (icon: Component) => () =>
  h(NIcon, { size: 20, class: 'sidebar-menu-icon' }, { default: () => h(icon) });

const menuOptions = computed(() => [
  { key: 'Home', label: '主页', icon: renderIcon(HomeOutline) },
  { key: 'CreateProxy', label: '新建隧道', icon: renderIcon(AddOutline), disabled: !session.signedIn },
  { key: 'ProxyList', label: '隧道管理', icon: renderIcon(BuildOutline) },
  { key: 'FrpcManager', label: '日志', icon: renderIcon(TerminalOutline) },
  { key: 'Network', label: '网络', icon: renderIcon(LogoWebComponent) },
  { key: 'Settings', label: '设置', icon: renderIcon(SettingsOutline) },
  { key: 'Info', label: '关于', icon: renderIcon(InformationOutline) },
  { key: 'webpanel', label: '打开网页面板', icon: renderIcon(EnterOutline) },
]);

async function handleMenuSelect(key: string) {
  if (key !== 'webpanel') {
    await router.push({ name: key });
    return;
  }
  const auth = authorization.value;
  const url = new URL(auth ? 'https://console.openfrp.net/fastlogin' : 'https://console.openfrp.net/');
  if (auth) url.searchParams.set('auth', auth);
  try {
    await openUrl(url.href);
  } catch {
    message.error('打开网页面板失败');
  }
}
</script>

<template>
  <n-layout-sider
    bordered collapse-mode="width" :collapsed-width="64" :width="240"
    :collapsed="collapsed" show-trigger
    @collapse="emit('update:collapsed', true)"
    @expand="emit('update:collapsed', false)"
  >
    <n-menu
      :collapsed="collapsed" :collapsed-width="64" :collapsed-icon-size="22"
      :options="menuOptions" :value="typeof route.name === 'string' ? route.name : null"
      @update:value="handleMenuSelect"
    />
  </n-layout-sider>
</template>

<style scoped>
:deep(.sidebar-menu-icon),
:deep(.sidebar-menu-icon svg) {
  width: 1em;
  height: 1em;
  display: inline-block;
  color: currentColor;
  fill: currentColor;
  flex-shrink: 0;
}
.n-layout-sider {
  background-color: var(--launcher-surface);
  backdrop-filter: blur(10px);
}
</style>
