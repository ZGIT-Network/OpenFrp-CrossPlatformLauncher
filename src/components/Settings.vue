<script setup lang="ts">
import { computed, h, inject, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import type { Ref } from 'vue';
import { NInput, useDialog, useMessage } from 'naive-ui';
import { HelpCircleOutline } from '@vicons/ionicons5';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { openUrl } from '@tauri-apps/plugin-opener';
import { mkdir, remove, writeFile } from '@tauri-apps/plugin-fs';
import { appDataDir, join } from '@tauri-apps/api/path';
import { useRoute } from 'vue-router';
import { v4 as uuid } from 'uuid';
import authhelpimage from '@/assets/authhelpimage.vue';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';
import { formatBandwidth, formatTraffic } from '@/utils/accountFormat';
import { globalLogService } from '@/services/logService';
import { frpcDownloading } from '@/services/tunnelRuntime';
import { useAppUpdates } from '@/services/appUpdates';
import type { UpdateInfo } from '@/services/appUpdates';
import { argoCancelWait, argoRequestLogin, argoWaitAuthorization } from '@/requests/argoAccess/request';

const message = useMessage();
const dialog = useDialog();
const route = useRoute();
const session = useSessionStore();
const { installing, installUpdate } = useAppUpdates();
const userInfo = computed(() => session.userInfo);
const avatarUrl = computed(() => {
  const email = userInfo.value?.email?.trim();
  return email ? `https://api.zyghit.cn/avatar/?${new URLSearchParams({ email, s: '256' })}` : undefined;
});
const activeNames = ref<string[]>(['2', '3', '4']);
watch(() => route.query.section, (section) => {
  if (section === 'frpc') activeNames.value = ['1'];
}, { immediate: true });

const currentVersion = ref('获取中...');
const checking = ref(false);
const logs = computed(() => globalLogService.entries.value
  .filter(entry => !entry.tunnel).slice(-150).map(entry => entry.text).join('\n'));
const expectedFrpcFilename = ref('');
const appDataDirPath = ref('');
const manualModeVisible = ref(false);
const frpcBusy = ref(false);
const downloading = frpcDownloading;

async function downloadFrpc() {
  if (downloading.value || frpcBusy.value) return;
  activeNames.value = ['1'];
  downloading.value = true;
  try {
    const result = await invoke<string>('download_frpc');
    message.success(result);
  } catch (error) {
    message.error(`下载失败：${errorMessage(error)}`);
  } finally {
    downloading.value = false;
  }
}
async function getFrpcVersion(): Promise<boolean> {
  if (frpcBusy.value || downloading.value) return false;
  frpcBusy.value = true;
  try {
    const version = await invoke<string>('get_frpc_cli_version');
    globalLogService.appendSystemLog(`本地 Frpc 版本：${version}`);
    message.success(`当前版本：${version}`);
    return true;
  } catch (error) {
    message.warning(errorMessage(error, '获取 Frpc 版本失败'));
    return false;
  } finally {
    frpcBusy.value = false;
  }
}
async function showManualMode() {
  if (frpcBusy.value) return;
  frpcBusy.value = true;
  try {
    const [dir, file] = await Promise.all([
      invoke<string>('get_app_data_dir'), invoke<{ filename: string }>('get_frpc_file'),
    ]);
    appDataDirPath.value = dir;
    expectedFrpcFilename.value = file.filename;
    manualModeVisible.value = true;
  } catch (error) {
    message.error(`获取 Frpc 路径失败：${errorMessage(error)}`);
  } finally {
    frpcBusy.value = false;
  }
}
async function openAppDataDir() {
  try { await invoke('open_app_data_dir'); }
  catch (error) { message.error(`打开数据目录失败：${errorMessage(error)}`); }
}
async function killAllProcesses() {
  try {
    await invoke('kill_all_processes');
    message.success('已终止启动器管理的 Frpc 进程');
  } catch (error) {
    message.error(`操作失败：${errorMessage(error)}`);
    return false;
  }
}
async function checkUpdate() {
  if (checking.value || installing.value) return;
  checking.value = true;
  try {
    const update = await invoke<UpdateInfo | null>('check_update');
    if (!update) {
      message.success('已是最新版本');
      return;
    }
    dialog.info({
      title: update.title,
      content: () => h('div', [
        `发现新版本 v${update.latest}`, h('br'), `当前版本 v${currentVersion.value}`,
        h('pre', { class: 'update-release-notes' }, update.msg),
      ]),
      positiveText: '立即更新',
      negativeText: '取消',
      onPositiveClick: () => { void installUpdate(update.latest); },
    });
  } catch (error) {
    message.error(`检查更新失败：${errorMessage(error)}`);
  } finally {
    checking.value = false;
  }
}

const helpDrawerContent = ref<'quickstart' | 'authorization'>();
const helpDrawerVisible = ref(false);
function helpDrawer(type: 'quickstart' | 'authorization') {
  helpDrawerContent.value = type;
  helpDrawerVisible.value = true;
}

type LoginMode = 'oauth' | 'argo' | 'authorization';
const loginMode = ref<LoginMode | null>(null);
const Authorization = ref('');
let generation = 0;
let argoId = '';
let oauthId = '';
let oauthReservation: Promise<string> | undefined;
let oauthWindow: WebviewWindow | undefined;
let oauthUnlisten: (() => void) | undefined;
let oauthCloseUnlisten: (() => void) | undefined;

function resetLogin() {
  generation++;
  loginMode.value = null;
  oauthUnlisten?.();
  oauthUnlisten = undefined;
  oauthCloseUnlisten?.();
  oauthCloseUnlisten = undefined;
  const window = oauthWindow;
  oauthWindow = undefined;
  if (window) void window.close().catch(console.error);
  if (oauthId) void invoke('cancel_local_oauth_server', { attemptId: oauthId }).catch(console.error);
  if (argoId) void argoCancelWait(argoId).catch(console.error);
  oauthId = '';
  argoId = '';
}

function cancelLogin() {
  resetLogin();
  message.info('已取消登录');
}

async function finishLogin(auth: string, attempt: number) {
  await session.signIn(auth, () => attempt === generation);
  if (attempt !== generation) return;
  Authorization.value = '';
  message.success('登录成功');
  resetLogin();
}

function loginError(error: unknown, attempt: number) {
  if (attempt !== generation) return;
  message.error(`登录失败：${errorMessage(error, '请稍后重试')}`);
  resetLogin();
}

interface OAuthResult {
  attemptId: string;
  flag: boolean;
  msg?: string;
  authorization?: string;
}

async function oauthLogin() {
  if (loginMode.value) return;
  loginMode.value = 'oauth';
  const attempt = ++generation;
  try {
    const unlisten = await listen<OAuthResult>('oauth-auth', ({ payload }) => {
      if (attempt !== generation || payload.attemptId !== oauthId) return;
      if (!payload.flag || !payload.authorization) {
        loginError(payload.msg || '未获得授权信息', attempt);
      } else {
        void finishLogin(payload.authorization, attempt).catch(error => loginError(error, attempt));
      }
    });
    if (attempt !== generation) {
      unlisten();
      return;
    }
    oauthUnlisten = unlisten;
    if (oauthReservation) {
      await oauthReservation.catch(() => {});
      if (attempt !== generation) return;
    }
    const reservation = invoke<string>('begin_local_oauth_login');
    oauthReservation = reservation;
    const attemptId = await reservation.finally(() => {
      if (oauthReservation === reservation) oauthReservation = undefined;
    });
    if (attempt !== generation) {
      await invoke('cancel_local_oauth_server', { attemptId });
      return;
    }
    oauthId = attemptId;
    const { loginUrl } = await invoke<{ loginUrl: string }>('start_local_oauth_server', { attemptId });
    if (attempt !== generation) return;
    async function openInBrowser() {
      if (attempt !== generation) return;
      try {
        await openUrl(loginUrl);
        if (attempt === generation) message.info('请在浏览器中完成授权');
      } catch {
        if (attempt !== generation) return;
        dialog.info({
          title: '手动登录',
          content: '请复制以下链接，在浏览器中打开后完成登录。',
          action: () => h(NInput, { value: loginUrl, readonly: true, onFocus: e => (e.target as HTMLInputElement)?.select() }),
        });
      }
    }
    if (await WebviewWindow.getByLabel('oauth-login')) {
      await openInBrowser();
      return;
    }
    if (attempt !== generation) return;
    try {
      const window = new WebviewWindow('oauth-login', {
        url: loginUrl, title: 'NatayarkID 登录', width: 520, height: 720, resizable: true, center: true,
      });
      oauthWindow = window;
      void window.once('tauri://error', () => {
        if (oauthWindow === window) oauthWindow = undefined;
        void openInBrowser();
      });
      void window.once('tauri://created', async () => {
        if (attempt !== generation) {
          await window.close().catch(console.error);
          return;
        }
        try {
          const unlistenClose = await window.onCloseRequested(() => {
            if (attempt === generation && oauthWindow === window) {
              oauthWindow = undefined;
              resetLogin();
            }
          });
          if (attempt !== generation) unlistenClose();
          else oauthCloseUnlisten = unlistenClose;
        } catch (error) {
          loginError(error, attempt);
        }
      });
    } catch {
      await openInBrowser();
    }
  } catch (error) {
    loginError(error, attempt);
  }
}

async function remoteLogin() {
  if (loginMode.value) return;
  loginMode.value = 'argo';
  const attempt = ++generation;
  try {
    const { request_uuid, authorization_url } = await argoRequestLogin();
    if (attempt !== generation) {
      await argoCancelWait(request_uuid);
      return;
    }
    argoId = request_uuid;
    const url = new URL(authorization_url);
    if (url.protocol !== 'https:' || url.hostname !== 'console.openfrp.net' || !url.searchParams.has('argoaccess')) {
      throw new Error('授权地址校验失败');
    }
    await openUrl(url.href);
    if (attempt !== generation) return;
    const auth = await argoWaitAuthorization(request_uuid);
    if (attempt !== generation) return;
    await finishLogin(auth, attempt);
    const window = getCurrentWindow();
    void window.show().then(() => window.setFocus()).catch(console.error);
  } catch (error) {
    loginError(error, attempt);
  }
}
async function authLogin() {
  if (loginMode.value) return;
  loginMode.value = 'authorization';
  const attempt = ++generation;
  try { await finishLogin(Authorization.value, attempt); }
  catch (error) { loginError(error, attempt); }
}
function logout() {
  dialog.warning({
    title: '确认退出',
    content: '确定要退出登录吗？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await session.signOut();
        message.success('已退出登录');
      } catch {
        message.warning('已退出本地登录，服务端会话注销失败');
      }
    },
  });
}
onUnmounted(resetLogin);

const autoStart = ref(false);
const autoStartLoading = ref(true);
const autoRestoreTunnels = ref(localStorage.getItem('autoRestoreTunnels') !== 'false');
const deepLinkEnabled = ref(false);
const deepLinkLoading = ref(true);
const canToggleDeepLink = !/mac/i.test(navigator.platform);
async function toggleAutoStart(enabled: boolean) {
  if (autoStartLoading.value) return;
  autoStartLoading.value = true;
  try {
    await invoke('toggle_auto_start', { enable: enabled });
    autoStart.value = enabled;
    message.success(`${enabled ? '启用' : '禁用'}开机自启动成功`);
  } catch (error) {
    message.error(`设置开机自启动失败：${errorMessage(error)}`);
  } finally {
    autoStartLoading.value = false;
  }
}
function toggleAutoRestoreTunnels(value: boolean) {
  autoRestoreTunnels.value = value;
  localStorage.setItem('autoRestoreTunnels', String(value));
}
async function debugAutoStart() {
  try {
    const info = await invoke<{ checks: { attempt: number; success: boolean; enabled: boolean; error?: string }[] }>('debug_auto_start');
    activeNames.value = ['1', '3'];
    for (const check of info.checks) {
      globalLogService.appendSystemLog(`自启动检查 ${check.attempt}：${check.success ? (check.enabled ? '启用' : '禁用') : check.error}`);
    }
    message.success('自启动调试信息已记录到日志');
  } catch (error) { message.error(errorMessage(error)); }
}
async function toggleDeepLink(enabled: boolean) {
  if (deepLinkLoading.value) return;
  deepLinkLoading.value = true;
  try {
    await invoke('set_quick_start_enabled', { enabled });
    deepLinkEnabled.value = enabled;
    message.success(`${enabled ? '启用' : '禁用'}快速启动功能成功`);
  } catch (error) {
    message.error(`设置快速启动失败：${errorMessage(error)}`);
  } finally {
    deepLinkLoading.value = false;
  }
}

const dohOptions = [
  { label: '默认 Ali (223.5.5.5)', value: '' },
  { label: 'DNSPod Public DNS (doh.pub)', value: 'doh.pub' },
  { label: 'Cloudflare (1.1.1.1)', value: '1.1.1.1' },
  { label: 'Ali (223.6.6.6)', value: '223.6.6.6' },
  { label: 'Google (8.8.8.8)', value: '8.8.8.8' },
  { label: '114DNS (114.114.114.114)', value: '114.114.114.114' },
  { label: 'Quad9 (9.9.9.9)', value: '9.9.9.9' },
];
const networkSettings = reactive({
  USE_DOH: 'false', DOH_ADDR: '', FRPC_FORCE_TLS: 'false', FRPC_DEBUG: 'false', BYPASS_PROXY: 'false',
});
type NetworkKey = keyof typeof networkSettings;
const saving = reactive(new Set<NetworkKey>());
const networkReady = ref(false);
async function saveNetworkSetting(key: NetworkKey, value: boolean | string) {
  if (!networkReady.value || saving.has(key)) return;
  saving.add(key);
  try {
    await invoke('set_env', { key, value: String(value) });
    networkSettings[key] = String(value);
  } catch (error) {
    message.error(`保存设置失败：${errorMessage(error)}`);
  } finally {
    saving.delete(key);
  }
}
onMounted(() => {
  void invoke<string>('get_cpl_version')
    .then(value => { currentVersion.value = value; }).catch(() => { currentVersion.value = '获取失败'; });
  void invoke<boolean>('check_auto_start')
    .then(value => { autoStart.value = value; }).catch(console.error).finally(() => { autoStartLoading.value = false; });
  void invoke<boolean>('get_quick_start_enabled')
    .then(value => { deepLinkEnabled.value = value; }).catch(console.error).finally(() => { deepLinkLoading.value = false; });
  const keys = Object.keys(networkSettings) as NetworkKey[];
  void Promise.all(keys.map(key => invoke<string | null>('get_env', { key })))
    .then(values => {
      keys.forEach((key, index) => { networkSettings[key] = values[index] ?? networkSettings[key]; });
      networkReady.value = true;
    }).catch(error => message.error(`读取网络设置失败：${errorMessage(error)}`));
});

const {
  backgroundImage, setBackgroundImage, backgroundBlur, setBackgroundBlur, backgroundOpacity, setBackgroundOpacity,
} = inject('backgroundImageControl') as {
  backgroundImage: Ref<string>; setBackgroundImage: (value: string) => void;
  backgroundBlur: Ref<number>; setBackgroundBlur: (value: number) => void;
  backgroundOpacity: Ref<number>; setBackgroundOpacity: (value: number) => void;
};
const fileInput = ref<HTMLInputElement | null>(null);
const backgroundBusy = ref(false);
function localImageName(value: string) {
  if (!/^(?:asset:\/\/localhost|https?:\/\/asset\.localhost)\//i.test(value)) return '';
  try {
    return decodeURIComponent(value.slice(value.indexOf('/', value.indexOf('://') + 3) + 1))
      .split(/[\\/]/).pop() || '';
  } catch { return ''; }
}
const displayBgUrl = computed(() => {
  const name = localImageName(backgroundImage.value);
  return name ? `本地文件：${name.replace(/^[a-f0-9-]{36}_/i, '')}` : backgroundImage.value;
});
async function removeOwnedImage(value: string) {
  const name = localImageName(value);
  if (!/^[a-f0-9-]{36}_.+/i.test(name)) return;
  try {
    const file = await join(await appDataDir(), 'backgrounds', name);
    if (convertFileSrc(file) === value) await remove(file);
  } catch (error) { console.error('清理旧背景失败:', error); }
}
async function clearBackgroundImage() {
  if (!backgroundImage.value || backgroundBusy.value) return;
  const old = backgroundImage.value;
  setBackgroundImage('');
  backgroundBusy.value = true;
  try { await removeOwnedImage(old); }
  finally { backgroundBusy.value = false; }
  message.success('背景已清除');
}
async function onFileChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file || backgroundBusy.value) return;
  if (!file.type.startsWith('image/') || file.size > 30 * 1024 * 1024) {
    message.warning('请选择 30 MB 以内的图片');
    return;
  }
  backgroundBusy.value = true;
  const preview = URL.createObjectURL(file);
  let stagedPath = '';
  try {
    const image = new Image();
    image.src = preview;
    await image.decode();
    const directory = await join(await appDataDir(), 'backgrounds');
    await mkdir(directory, { recursive: true });
    stagedPath = await join(directory, `${uuid()}_${file.name}`);
    await writeFile(stagedPath, new Uint8Array(await file.arrayBuffer()));
    const old = backgroundImage.value;
    setBackgroundImage(convertFileSrc(stagedPath));
    stagedPath = '';
    await removeOwnedImage(old);
    message.success('背景图片已保存');
  } catch {
    if (stagedPath) await remove(stagedPath).catch(console.error);
    message.error('保存图片失败');
  } finally {
    URL.revokeObjectURL(preview);
    backgroundBusy.value = false;
  }
}
</script>

<template>
  <n-space vertical :size="12">
    <n-h2 style="margin-bottom: 0;">设置</n-h2>
    <n-alert type="warning">
      您当前正在使用 Beta 测试版本，可能存在一些问题，请谨慎在生产环境使用。
      <br />若遇到问题，请及时反馈。
    </n-alert>

    <n-card v-if="session.signedIn" title="已登录">
      <template #header-extra><n-button tertiary @click="logout">退出登录</n-button></template>
      <n-space v-if="userInfo" vertical>
        <n-thing>
          <template #avatar>
            <n-tooltip :show-arrow="false" placement="bottom">
              <template #trigger>
                <n-avatar :size="48" :src="avatarUrl" :img-props="{ alt: '用户头像', referrerpolicy: 'no-referrer' }">
                  <template v-if="!avatarUrl" #default>{{ userInfo.username?.slice(0, 1) }}</template>
                  <template #fallback>{{ userInfo.username?.slice(0, 1) }}</template>
                </n-avatar>
              </template>
              更换头像？请前往 Weavatar !
            </n-tooltip>
          </template>
          <template #header>
            #{{ userInfo.id }} {{ userInfo.username }}
            <n-tag type="info" size="small">{{ userInfo.friendlyGroup }}</n-tag>
          </template>
          <template #description>{{ userInfo.email }}</template>
        </n-thing>
        <n-descriptions :column="2">
          <n-descriptions-item label="隧道数">{{ userInfo.used }} / {{ userInfo.proxies }}</n-descriptions-item>
          <n-descriptions-item label="带宽速率 (上 / 下)">
            {{ formatBandwidth(userInfo.outLimit) }} / {{ formatBandwidth(userInfo.inLimit) }}
          </n-descriptions-item>
          <n-descriptions-item label="注册时间">{{ userInfo.regTime }}</n-descriptions-item>
          <n-descriptions-item label="剩余流量">{{ formatTraffic(userInfo.traffic) }}</n-descriptions-item>
        </n-descriptions>
      </n-space>
      <n-space v-else-if="session.profileError" justify="space-between" align="center">
        <n-text type="warning">{{ session.profileError }}</n-text>
        <n-button size="small" :loading="session.loading" @click="session.refreshProfile">重试</n-button>
      </n-space>
      <n-skeleton v-else :height="80" />
    </n-card>

    <n-card v-else title="用户登录">
      <n-tabs type="bar" animated>
        <n-tab-pane name="oauth" tab="通过 NatayarkID 登录">
          <n-space align="center">
            <n-button type="primary" :loading="loginMode === 'oauth'" :disabled="!!loginMode" @click="oauthLogin">OAuth 登录</n-button>
            <n-button v-if="loginMode === 'oauth'" @click="cancelLogin">取消</n-button>
          </n-space>
        </n-tab-pane>
        <n-tab-pane name="remote" tab="远程安全登录 (Argo)">
          <n-space vertical>
            <n-text depth="3">将生成一次性密钥并请求授权，需在浏览器完成确认。</n-text>
            <n-space>
              <n-button type="primary" :loading="loginMode === 'argo'" :disabled="!!loginMode" @click="remoteLogin">开始远程登录</n-button>
              <n-button :disabled="loginMode !== 'argo'" @click="cancelLogin">取消</n-button>
            </n-space>
            <n-text v-if="loginMode === 'argo'">正在等待授权（最长 5 分钟）...</n-text>
          </n-space>
        </n-tab-pane>
        <n-tab-pane name="authorization" tab="通过 Authorization 登录（已弃用）">
          <n-form @submit.prevent="authLogin">
            <n-form-item label="请输入在面板获取的 Authorization">
              <n-input v-model:value="Authorization" type="password" placeholder="Authorization" :disabled="!!loginMode" />
              <n-button quaternary circle aria-label="Authorization 登录帮助" @click="helpDrawer('authorization')">
                <template #icon><n-icon><HelpCircleOutline /></n-icon></template>
              </n-button>
            </n-form-item>
            <n-button type="primary" attr-type="submit" :loading="loginMode === 'authorization'" :disabled="!!loginMode">登录</n-button>
          </n-form>
        </n-tab-pane>
      </n-tabs>
    </n-card>

    <n-card>
      <n-collapse v-model:expanded-names="activeNames">
        <n-collapse-item title="版本信息" name="2">
          <n-space vertical>
            <n-text>当前版本：Beta v{{ currentVersion }}</n-text>
            <n-space>
              <n-button @click="checkUpdate" :loading="checking || installing">检查更新</n-button>
              <n-button @click="openAppDataDir">打开软件数据目录</n-button>
            </n-space>
          </n-space>
        </n-collapse-item>
        <n-collapse-item title="Frpc 管理" name="1">
          <n-space vertical>
            <n-space align="center">
              <n-switch :value="networkSettings.FRPC_DEBUG === 'true'" :disabled="!networkReady"
                :loading="saving.has('FRPC_DEBUG')" aria-label="启用 Frpc Debug 模式"
                @update:value="saveNetworkSetting('FRPC_DEBUG', $event)" />
              <span>启用 Frpc Debug 模式</span>
            </n-space>
            <n-space>
              <n-button @click="downloadFrpc" :loading="downloading" :disabled="frpcBusy">自动下载/更新 Frpc</n-button>
              <n-button @click="getFrpcVersion" :loading="frpcBusy" :disabled="downloading">获取本地 Frpc 版本</n-button>
              <n-button @click="showManualMode" :disabled="frpcBusy || downloading">手动配置 Frpc 可执行文件</n-button>
              <n-popconfirm @positive-click="killAllProcesses" :disabled="downloading">
                <template #trigger><n-button type="warning" :disabled="downloading">终止所有 Frpc 进程</n-button></template>
                确认终止启动器管理的 Frpc 进程？这将断开对应的隧道连接。
              </n-popconfirm>
            </n-space>
            <n-card title="运行日志"><n-log :rows="8" :log="logs" trim /></n-card>
          </n-space>
        </n-collapse-item>

        <n-collapse-item title="启动器设置" name="3">
          <n-space vertical>
            <n-space align="center">
              <n-switch :value="autoStart" :loading="autoStartLoading" aria-label="开机自启动" @update:value="toggleAutoStart" />
              <span>开机自启动</span>
              <n-button size="small" type="info" @click="debugAutoStart">调试状态</n-button>
            </n-space>
            <n-space v-if="autoStart" align="center">
              <n-switch :value="autoRestoreTunnels" aria-label="开机时恢复上次运行的隧道" @update:value="toggleAutoRestoreTunnels" />
              <span>开机时恢复上次运行的隧道</span>
            </n-space>
            <n-space align="center">
              <n-switch :value="deepLinkEnabled" :disabled="!canToggleDeepLink" :loading="deepLinkLoading"
                aria-label="启用快速启动功能" @update:value="toggleDeepLink" />
              <span>{{ canToggleDeepLink ? '启用“快速启动”功能' : '“快速启动”链接由 macOS 管理' }}</span>
              <n-button quaternary circle aria-label="快速启动帮助" @click="helpDrawer('quickstart')">
                <template #icon><n-icon><HelpCircleOutline /></n-icon></template>
              </n-button>
            </n-space>
            <n-divider title-placement="left" dashed style="margin: 4px 0;">个性化设置</n-divider>
            <n-space align="center">
              <n-slider :value="backgroundBlur" :min="20" :max="100" :step="1" class="background-slider"
                :disabled="!backgroundImage" aria-label="背景模糊" @update:value="setBackgroundBlur" />
              <span>{{ backgroundBlur }}% 模糊</span>
            </n-space>
            <n-space align="center">
              <n-slider :value="backgroundOpacity" :min="0" :max="100" :step="1" class="background-slider"
                :disabled="!backgroundImage" aria-label="背景不透明度" @update:value="setBackgroundOpacity" />
              <span>{{ backgroundOpacity }}% 不透明度</span>
            </n-space>
            <n-space align="center">
              <n-input v-if="displayBgUrl" :value="displayBgUrl" readonly placeholder="背景图片地址" class="background-address" />
              <n-button :loading="backgroundBusy" @click="fileInput?.click()">设置背景图</n-button>
              <input ref="fileInput" type="file" accept="image/*" hidden @change="onFileChange" />
              <n-button :disabled="!backgroundImage || backgroundBusy" @click="clearBackgroundImage">清除背景</n-button>
            </n-space>
          </n-space>
        </n-collapse-item>

        <n-collapse-item title="网络设置" name="4">
          <n-space vertical>
            <n-space align="center">
              <n-switch :value="networkSettings.USE_DOH === 'true'" :disabled="!networkReady" :loading="saving.has('USE_DOH')"
                aria-label="启用 DoH 解析" @update:value="saveNetworkSetting('USE_DOH', $event)" />
              <span>启用 DoH 解析</span>
              <n-tooltip>
                <template #trigger><n-icon><HelpCircleOutline /></n-icon></template>
                启用后，Frpc 将使用基于 HTTPS 的 DNS 解析。
              </n-tooltip>
            </n-space>
            <n-space align="center">
              选择 DoH 服务器
              <n-select :value="networkSettings.DOH_ADDR" :options="dohOptions" class="doh-select"
                :disabled="!networkReady || networkSettings.USE_DOH !== 'true' || saving.has('USE_DOH')"
                :loading="saving.has('DOH_ADDR')" @update:value="saveNetworkSetting('DOH_ADDR', $event)" />
            </n-space>
            <n-space align="center">
              <n-switch :value="networkSettings.FRPC_FORCE_TLS === 'true'" :disabled="!networkReady"
                :loading="saving.has('FRPC_FORCE_TLS')" aria-label="启用 Force TLS"
                @update:value="saveNetworkSetting('FRPC_FORCE_TLS', $event)" />
              <span>启用 Force TLS</span>
              <n-tooltip>
                <template #trigger><n-icon><HelpCircleOutline /></n-icon></template>
                强制启用与节点服务器的通信加密。
              </n-tooltip>
            </n-space>
            <n-space align="center">
              <n-switch :value="networkSettings.BYPASS_PROXY === 'true'" :disabled="!networkReady"
                :loading="saving.has('BYPASS_PROXY')" aria-label="绕过系统代理"
                @update:value="saveNetworkSetting('BYPASS_PROXY', $event)" />
              <span>绕过系统代理</span>
              <n-tooltip>
                <template #trigger><n-icon><HelpCircleOutline /></n-icon></template>
                用于直接访问 API 并绕过系统代理。
              </n-tooltip>
            </n-space>
          </n-space>
        </n-collapse-item>
      </n-collapse>
    </n-card>
  </n-space>

  <n-drawer v-model:show="helpDrawerVisible" width="min(480px, 100vw)" placement="right">
    <n-drawer-content closable title="功能帮助指南">
      <n-thing v-if="helpDrawerContent === 'quickstart'">
        <n-h3>快速启动</n-h3>
        <n-text>在网页面板点击快速启动链接可直接启动隧道。快速启动的隧道不会随开机恢复。</n-text>
      </n-thing>
      <n-thing v-else>
        <n-h3>通过 Authorization 登录</n-h3>
        <n-text>可在网页面板的个人中心，通过「第三方客户端安全登录」获取 Authorization。</n-text>
        <authhelpimage />
      </n-thing>
    </n-drawer-content>
  </n-drawer>
  <n-drawer v-model:show="manualModeVisible" width="min(600px, 100vw)" placement="right">
    <n-drawer-content title="手动配置 Frpc 可执行文件" closable>
      <n-space vertical>
        <n-alert type="info">
          从 OpenFrp 下载中心下载与当前系统和架构匹配的 Frpc，解压后按下方文件名放入应用数据目录。
        </n-alert>
        <n-form-item label="应用数据目录"><n-input :value="appDataDirPath" readonly /></n-form-item>
        <n-button @click="openAppDataDir">打开数据目录</n-button>
        <n-form-item label="Frpc 可执行文件名称"><n-input :value="expectedFrpcFilename" readonly /></n-form-item>
      </n-space>
      <template #footer>
        <n-space>
          <n-button @click="manualModeVisible = false">关闭</n-button>
          <n-button type="primary" :loading="frpcBusy" @click="async () => { if (await getFrpcVersion()) manualModeVisible = false }">
            完成并检查
          </n-button>
        </n-space>
      </template>
    </n-drawer-content>
  </n-drawer>
</template>

<style scoped>
.background-slider { width: min(200px, 55vw); }
.background-address { width: min(320px, 55vw); }
.doh-select { width: min(260px, 55vw); }
</style>
<style>
.update-release-notes {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  max-height: 50vh;
  overflow: auto;
}
</style>
