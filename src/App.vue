<script setup lang="ts">
import { computed, onMounted, onUnmounted, provide, ref, watch } from 'vue';
import { dateZhCN, darkTheme, lightTheme, useOsTheme, zhCN } from 'naive-ui';
import type { GlobalThemeOverrides } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useRoute } from 'vue-router';
import Header from './layouts/Header/index.vue';
import Sidebar from './layouts/Sidebar/index.vue';
import { globalLogService } from '@/services/logService';
import { recordStoppedTunnels, refreshRunningTunnels } from '@/services/tunnelRuntime';
import type { TunnelInstance } from '@/services/tunnelRuntime';
import { useSessionStore } from '@/stores/session';

const session = useSessionStore();
const isDev = import.meta.env.DEV;
const route = useRoute();
const pageScroll = ref<HTMLElement | null>(null);
watch(() => route.path, () => pageScroll.value?.scrollTo(0, 0));

const osTheme = useOsTheme();
const savedScheme = localStorage.getItem('colorScheme');
const selectedScheme = ref<'light' | 'dark' | null>(
  savedScheme === 'dark' || savedScheme === 'light' ? savedScheme : null
);
const colorScheme = computed(() => selectedScheme.value ?? (osTheme.value === 'dark' ? 'dark' : 'light'));
const toggleColorScheme = () => {
  selectedScheme.value = colorScheme.value === 'dark' ? 'light' : 'dark';
  localStorage.setItem('colorScheme', selectedScheme.value);
};
provide('darkMode', { colorScheme, toggleColorScheme });
watch(colorScheme, (scheme) => {
  document.documentElement.classList.toggle('actual-dark', scheme === 'dark');
  document.documentElement.classList.toggle('actual-light', scheme === 'light');
}, { immediate: true });
const theme = computed(() => colorScheme.value === 'dark' ? darkTheme : lightTheme);

const collapsed = ref(window.innerWidth <= 900);
const updateSidebar = (event: MediaQueryListEvent) => { collapsed.value = event.matches; };
const narrowWindow = window.matchMedia('(max-width: 900px)');

function settingNumber(key: string, fallback: number, minimum: number) {
  const value = Number(localStorage.getItem(key) ?? fallback);
  return Number.isFinite(value) ? Math.max(minimum, Math.min(100, value)) : fallback;
}
const backgroundImage = ref(localStorage.getItem('backgroundImage') || '');
const backgroundBlur = ref(settingNumber('backgroundBlur', 40, 20));
const backgroundOpacity = ref(settingNumber('backgroundOpacity', 100, 0));
const setBackgroundImage = (url: string) => {
  if (url) localStorage.setItem('backgroundImage', url);
  else localStorage.removeItem('backgroundImage');
  backgroundImage.value = url;
};
const setBackgroundBlur = (value: number) => {
  backgroundBlur.value = value;
  localStorage.setItem('backgroundBlur', String(value));
};
const setBackgroundOpacity = (value: number) => {
  backgroundOpacity.value = value;
  localStorage.setItem('backgroundOpacity', String(value));
};
provide('backgroundImageControl', {
  backgroundImage, setBackgroundImage, backgroundBlur, setBackgroundBlur, backgroundOpacity, setBackgroundOpacity,
});
const backgroundStyle = computed(() => ({
  backgroundImage: `url(${JSON.stringify(backgroundImage.value)})`,
  filter: `blur(${backgroundBlur.value / 5}px)`,
  opacity: backgroundOpacity.value / 100,
}));

const currentVersion = ref('');
const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const dark = colorScheme.value === 'dark';
  return {
    common: {
      primaryColor: dark ? '#72a0c9' : '#3974a7',
      primaryColorHover: dark ? '#529bdb' : '#2c6495',
      primaryColorPressed: '#0F6FB8',
      primaryColorSuppl: '#529bdb',
      borderRadius: '5px',
    },
    Layout: {
      headerBorderColor: dark ? 'rgba(255, 255, 255, 0.25)' : 'rgba(0, 0, 0, 0.18)',
      siderBorderColor: dark ? 'rgba(255, 255, 255, 0.25)' : 'rgba(0, 0, 0, 0.18)',
    },
    Menu: { itemColorHover: dark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.09)' },
  };
});

let disposed = false;
const cleanups: (() => void)[] = [];
const start = (registration: Promise<() => void>) => {
  void registration.then((unlisten) => {
    if (disposed) unlisten();
    else cleanups.push(unlisten);
  }).catch(console.error);
};

onMounted(() => {
  void session.refreshProfile();
  const timer = setInterval(() => void session.refreshProfile(), 10 * 60 * 1000);
  narrowWindow.addEventListener('change', updateSidebar);
  window.addEventListener('pagehide', flushLogs);
  cleanups.push(() => {
    clearInterval(timer);
    narrowWindow.removeEventListener('change', updateSidebar);
    window.removeEventListener('pagehide', flushLogs);
  });
  start(globalLogService.attach());
  start(listen<TunnelInstance[]>('tunnels-stopped', ({ payload }) => {
    recordStoppedTunnels(payload);
    void refreshRunningTunnels().catch(console.error);
  }));
  void refreshRunningTunnels().catch(console.error);
  void invoke<string>('get_cpl_version').then(version => { currentVersion.value = version; }).catch(console.error);
});

function flushLogs() {
  globalLogService.flush();
}

onUnmounted(() => {
  disposed = true;
  cleanups.forEach(cleanup => cleanup());
});
</script>

<template>
  <n-watermark v-if="isDev" content="OFCPL 开发模式 | 开发中功能，不代表最终效果"
    cross fullscreen :font-size="16" :line-height="80" :width="600" :height="800"
    :x-offset="12" :y-offset="60" :rotate="-12" style="z-index: 99999; pointer-events: none" />
  <div v-if="backgroundImage" class="app-background" :style="backgroundStyle" aria-hidden="true" />
  <n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN" :theme="theme">
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <n-loading-bar-provider>
          <n-notification-provider>
            <div class="app-shell">
              <n-layout-header bordered class="app-header">
                <Header />
              </n-layout-header>
              <n-layout has-sider position="absolute" class="app-main" style="top: 48px;" content-style="height: 100%; overflow: hidden;">
                <Sidebar v-model:collapsed="collapsed" />
                <main ref="pageScroll" class="app-page-scroll">
                  <div class="app-page">
                    <router-view v-slot="{ Component, route: currentRoute }">
                      <Transition name="page-switch" mode="out-in">
                        <div v-if="Component" :key="currentRoute.path">
                          <component :is="Component" />
                        </div>
                      </Transition>
                    </router-view>
                  </div>
                  <n-text v-if="currentVersion" tag="footer" depth="3" class="app-version">
                    OpenFrp Cross Platform Launcher · Beta v{{ currentVersion }} 预览体验计划
                  </n-text>
                </main>
              </n-layout>
            </div>
          </n-notification-provider>
        </n-loading-bar-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
html, body, #app {
  margin: 0;
  width: 100%;
  height: 100%;
}
html {
  overflow: hidden;
  color-scheme: light;
  --launcher-base: #f7f7f7;
  --launcher-surface: rgba(255, 255, 255, 0.4);
  --launcher-main: rgba(255, 255, 255, 0.2);
  --launcher-link: #3974a7;
  --launcher-selection: rgba(57, 116, 167, 0.12);
}
html.actual-dark {
  color-scheme: dark;
  --launcher-base: #202020;
  --launcher-surface: rgba(30, 30, 30, 0.4);
  --launcher-main: rgba(30, 30, 30, 0.3);
  --launcher-link: #72a0c9;
  --launcher-selection: rgba(114, 160, 201, 0.16);
}
#app {
  position: relative;
  isolation: isolate;
  background-color: var(--launcher-base);
}
.app-background {
  position: absolute;
  inset: -20px;
  z-index: -1;
  background-position: center;
  background-size: cover;
  pointer-events: none;
}
.app-shell {
  height: 100vh;
  height: 100dvh;
  position: relative;
}
.app-header.n-layout-header {
  height: 48px;
  padding: 0;
  background-color: var(--launcher-surface);
}
.app-main.n-layout {
  background-color: var(--launcher-main);
}
.app-page-scroll {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: auto;
}
.app-page {
  flex: 1 0 auto;
  padding: 20px 24px 24px;
  min-width: 0;
  overflow-wrap: anywhere;
}
.app-page h2:first-child {
  margin-top: 0;
}
@media (prefers-reduced-motion: no-preference) {
  .page-switch-enter-active { transition: opacity 160ms ease-out, transform 160ms ease-out; }
  .page-switch-leave-active { transition: opacity 90ms ease-in; }
  .page-switch-enter-from { opacity: 0; transform: translateY(4px); }
  .page-switch-leave-to { opacity: 0; }
}
.app-version {
  align-self: flex-end;
  text-align: right;
  font-size: 12px;
  padding: 0 24px 16px;
}
.n-card {
  background-color: var(--launcher-surface) !important;
  backdrop-filter: blur(8px);
}
a { text-decoration: none; color: var(--launcher-link); }
a:hover { color: #529bdb; }
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-thumb { background: rgba(0, 0, 0, 0.18); border-radius: 999px; }
::-webkit-scrollbar-thumb:hover { background: rgba(0, 0, 0, 0.32); }
.actual-dark ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.22); }
.actual-dark ::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.36); }

@media (max-width: 900px) {
  .app-page { padding: 16px; }
  .app-version { padding-inline: 16px; }
}
</style>
