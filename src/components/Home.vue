<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useRouter } from 'vue-router';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import copy from 'copy-to-clipboard';
import dayjs from 'dayjs';
import { CalendarOutline, KeyOutline, PersonCircleOutline } from '@vicons/ionicons5';
import { useSessionStore } from '@/stores/session';
import { authorization, errorMessage } from '@/utils/apiClient';
import { formatBandwidth, formatTraffic } from '@/utils/accountFormat';
import frpApiGetSignInfo from '@/requests/frpApi/frpApiGetSignInfo';

interface Announcement {
  cpl_announce?: string;
  headalert?: { status?: boolean; title?: string; type?: 'info' | 'success' | 'warning' | 'error'; content?: string };
}

const message = useMessage();
const router = useRouter();
const session = useSessionStore();
const announcement = ref<Announcement>();
const announcementLoading = ref(false);
const announcementError = ref('');
const signed = ref(false);

const sanitize = (html: string) => DOMPurify.sanitize(html, {
  USE_PROFILES: { html: true },
  FORBID_TAGS: ['form', 'input', 'button', 'video', 'audio'],
  FORBID_ATTR: ['style'],
});
const announcementHtml = computed(() => {
  const content = announcement.value?.cpl_announce;
  return typeof content === 'string' ? sanitize(marked.parse(content, { async: false })) : '';
});
const headAlert = computed(() => announcement.value?.headalert);
const headAlertHtml = computed(() => sanitize(headAlert.value?.content || ''));

async function fetchAnnouncement() {
  if (announcementLoading.value) return;
  announcementLoading.value = true;
  announcementError.value = '';
  try {
    announcement.value = await invoke<Announcement>('get_announcement');
  } catch (error) {
    announcementError.value = errorMessage(error, '获取公告失败');
  } finally {
    announcementLoading.value = false;
  }
}
onMounted(() => { void fetchAnnouncement(); });

watch(() => session.userInfo?.id, async (id, _, onCleanup) => {
  let current = true;
  onCleanup(() => { current = false; });
  signed.value = false;
  if (id == null) return;
  const result = await frpApiGetSignInfo();
  if (current && result.flag && result.data?.signdate) {
    const stamp = result.data.signdate;
    const signTime = dayjs(typeof stamp === 'number' && stamp < 1e12 ? stamp * 1000 : stamp);
    signed.value = signTime.isSame(dayjs(), 'day');
  }
}, { immediate: true });

async function openLink(url: string) {
  try {
    await openUrl(url);
  } catch {
    message.error('打开网页失败');
  }
}
function userSign() {
  const auth = authorization.value;
  const url = new URL(auth ? 'https://console.openfrp.net/fastlogin' : 'https://console.openfrp.net/');
  if (auth) url.searchParams.set('auth', auth);
  url.searchParams.set('type', 'sign');
  return openLink(url.href);
}
function copyToken() {
  const token = session.userInfo?.token;
  if (token && copy(token)) message.success('已复制至剪贴板，请妥善保管。');
  else message.error('复制访问密钥失败');
}
</script>

<template>
  <div>
    <n-h2 style="margin-bottom: 3px;">欢迎回来</n-h2>
    欢迎使用全新 <n-gradient-text type="info">OpenFrp Cross Platform Launcher</n-gradient-text> !

    <n-space vertical style="margin-top: 10px;">
      <n-alert v-if="headAlert?.status" :title="headAlert.title" :type="headAlert.type || 'info'" closable>
        <div class="markdown" v-html="headAlertHtml" v-external />
      </n-alert>

      <n-card v-if="!session.signedIn" :bordered="false" style="text-align: center;">
        <n-flex vertical align="center" justify="center" :size="12">
          <n-icon size="64" color="#f0a020"><PersonCircleOutline /></n-icon>
          <n-h4 style="margin: 0;">您尚未登录</n-h4>
          <n-text depth="3">登录后可同步配置、管理隧道并解锁更多功能</n-text>
          <n-space :size="16" style="margin-top: 8px;">
            <n-button type="primary" @click="router.push('/settings')">立即登录</n-button>
            <n-button tertiary @click="openLink('https://account.naids.com/register')">立即注册</n-button>
          </n-space>
        </n-flex>
      </n-card>

      <n-card v-else>
        <n-skeleton v-if="!session.userInfo && !session.profileError" height="120px" :sharp="false" />
        <n-alert v-else-if="!session.userInfo" type="warning" title="获取用户信息失败">
          <n-space align="center" justify="space-between">
            <n-text>{{ session.profileError }}</n-text>
            <n-button size="small" :loading="session.loading" @click="session.refreshProfile">重试</n-button>
          </n-space>
        </n-alert>
        <n-space v-else vertical>
          <n-descriptions :column="2">
            <n-descriptions-item label="主权限组">{{ session.userInfo.friendlyGroup }}</n-descriptions-item>
            <n-descriptions-item label="隧道数">{{ session.userInfo.used }} / {{ session.userInfo.proxies }}</n-descriptions-item>
            <n-descriptions-item label="注册邮箱">{{ session.userInfo.email }}</n-descriptions-item>
            <n-descriptions-item label="带宽速率 (上 / 下)">
              {{ formatBandwidth(session.userInfo.outLimit) }} / {{ formatBandwidth(session.userInfo.inLimit) }}
            </n-descriptions-item>
            <n-descriptions-item label="注册时间">{{ session.userInfo.regTime }}</n-descriptions-item>
            <n-descriptions-item label="剩余流量">{{ formatTraffic(session.userInfo.traffic) }}</n-descriptions-item>
          </n-descriptions>
          <n-alert v-if="!session.userInfo.realname" type="warning">完成实名认证后可使用中国大陆节点。</n-alert>
          <n-text v-if="!signed" depth="3">签到可以获得免费流量，打开网页面板立即完成每日签到。</n-text>
          <n-space :size="24">
            <n-button v-if="session.userInfo.token" type="warning" text strong @click="copyToken">
              <template #icon><n-icon><KeyOutline /></n-icon></template>
              复制访问密钥
            </n-button>
            <n-button v-if="!signed" type="success" text @click="userSign">
              <template #icon><n-icon><CalendarOutline /></n-icon></template>
              签到
            </n-button>
          </n-space>
        </n-space>
      </n-card>

      <n-card title="系统公告">
        <n-skeleton v-if="announcementLoading" :height="40" />
        <n-space v-else-if="announcementError" align="center" justify="space-between">
          <n-text depth="3">{{ announcementError }}</n-text>
          <n-button size="small" @click="fetchAnnouncement">重试</n-button>
        </n-space>
        <div v-else-if="announcementHtml" class="markdown" v-html="announcementHtml" v-external />
        <n-text v-else depth="3">暂无公告</n-text>
      </n-card>
    </n-space>
  </div>
</template>

<style scoped>
.markdown :deep(p:first-child) { margin-top: 0; }
.markdown :deep(p:last-child) { margin-bottom: 0; }
.markdown :deep(img) { max-width: 100%; height: auto; }
.markdown :deep(pre) {
  margin: 10px 0;
  padding: 12px 16px;
  overflow: auto;
  color: #abb2bf;
  background: #282c34;
  border-radius: 5px;
}
</style>
