<script setup lang="ts">
import { inject, Ref, ref, watch, onMounted, onUnmounted, computed } from 'vue';
import {
  NAlert,
  NGradientText,
  useMessage,
  NCard,
  NSpace,
  useNotification,
  NButton,
  NResult,
  NFlex,
  NText,
  NTooltip,
  NDescriptions,
  NDescriptionsItem,
  NSkeleton,
} from 'naive-ui';
import axios from 'axios';
import { marked } from 'marked';
import copy from 'copy-to-clipboard';
import dayjs from 'dayjs';
import { useRouter } from 'vue-router';
import { openUrl } from '@tauri-apps/plugin-opener';

// utils & api
import Cookies from '@/utils/cookies';
import frpApiGetUserInfo from '@/requests/frpApi/frpApiGetUserInfo';
import frpApiGetSignInfo from '@/requests/frpApi/frpApiGetSignInfo';

// icons
import { CalendarOutline, KeyOutline, PersonCircleOutline } from '@vicons/ionicons5';

import numbro from 'numbro';

/**
 * ------------------------------------------------------------------
 *  State
 * ------------------------------------------------------------------
 */
const message = useMessage();
const notification = useNotification();
const router = useRouter();

// 用户信息由父级(大概率是 App.vue) provide，此处 inject
const userInfoObj = inject<{
  userInfo: Ref<Struct.UserInfo | undefined>;
  getUserInfo: () => void;
}>('userInfo');

const userInfo = userInfoObj?.userInfo;

const loading = ref(true); // 页面整体 loading
const username = ref('');
const signInfo = ref<Struct.SignInfo>();
const signed = ref(false);

// 公告相关
const announcement = ref('加载中...');
const headAlert = ref<any>();

/**
 * ------------------------------------------------------------------
 *  Computed & utils
 * ------------------------------------------------------------------
 */
const isLoggedIn = computed(() => !!userInfo?.value?.token);

const byteFormat = (num: number) =>
  numbro(num * 1024 * 8)
    .format({ output: 'byte', base: 'general' })
    .replace('B', 'b');

const goToLogin = () => {
  router.push('/settings');
  message.info('请在设置页面完成登录');
};

const userSign = () => {
  try {
    const auth = Cookies.get('authorization');
    const url = auth
      ? `https://console.openfrp.net/fastlogin?auth=${auth}&type=sign`
      : 'https://console.openfrp.net/?type=sign';
    openUrl(url);
  } catch (error) {
    console.error('打开网页面板失败:', error);
  }
};

/**
 * ------------------------------------------------------------------
 *  API wrappers
 * ------------------------------------------------------------------
 */
const fetchAnnouncement = async () => {
  try {
    const { data } = await axios.get(
      'https://of-dev-api.bfsea.com/commonQuery/get',
      {
        params: { key: 'help_info' },
      }
    );
    headAlert.value = data.data.headalert;
    announcement.value = data.data.cpl_announce;
  } catch (e) {
    message.error(`获取公告失败: ${e}`);
  }
};

const fetchUserInfo = async (retry = 2) => {
  if (!isLoggedIn.value) {
    loading.value = false;
    return;
  }
  // 已有数据直接返回
  if (userInfo?.value) {
    loading.value = false;
    return;
  }
  try {
    const res = await frpApiGetUserInfo();
    if (res.flag) {
      userInfoObj && (userInfoObj.userInfo.value = res.data);
    } else if (retry) {
      // 重试
      return await fetchUserInfo(retry - 1);
    } else {
      message.error('获取用户信息失败');
    }
  } catch (err) {
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const fetchSignInfo = async () => {
  try {
    const res = await frpApiGetSignInfo();
    if (res.flag) {
      signInfo.value = res.data;
      signed.value = dayjs(res.data.signdate).diff(dayjs().startOf('day')) >= 0;
    } else {
      message.error(res.msg);
    }
  } catch (err) {
    console.error(err);
  }
};

/**
 * ------------------------------------------------------------------
 *  Watchers
 * ------------------------------------------------------------------
 */
if (userInfoObj?.userInfo) {
  watch(
    userInfoObj.userInfo,
    newVal => {
      if (newVal) {
        username.value = newVal.username ?? '';
        loading.value = false;
      }
    },
    { immediate: true }
  );
}

/**
 * ------------------------------------------------------------------
 *  Helper
 * ------------------------------------------------------------------
 */
const checkRealname = () => {
  if (!userInfo?.value?.realname) {
    message.warning('您尚未实名认证，请尽快进行实名认证以体验最佳服务');
    notification.error({
      title: '您尚未完成实名认证',
      content:
        '请尽快进行实名认证以获得最佳服务体验。未实名将无法使用中国大陆地区节点。',
      keepAliveOnHover: true,
      closable: true,
    });
  }
};

const init = async () => {
  await Promise.all([fetchAnnouncement(), fetchUserInfo()]);

  if (isLoggedIn.value) {
    await fetchSignInfo();
    checkRealname();
  }
};

/**
 * ------------------------------------------------------------------
 *  Refresh helper (页面级刷新动画)
 * ------------------------------------------------------------------
 */
const refreshEntireWindow = () => {
  const transitionEl = document.createElement('div');
  Object.assign(transitionEl.style, {
    position: 'fixed',
    inset: '0',
    backgroundColor: '#121212',
    zIndex: '9999',
    opacity: '0',
    transition: 'opacity .3s ease',
  });

  const spinner = document.createElement('div');
  Object.assign(spinner.style, {
    width: '40px',
    height: '40px',
    // border: '4px solid rgba(255,255,255,.1)',
    borderRadius: '50%',
    borderTopColor: '#18a058',
    position: 'absolute',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%,-50%)',
    animation: 'spin 1s linear infinite',
  });
  transitionEl.appendChild(spinner);
  document.body.appendChild(transitionEl);

  // 动效
  setTimeout(() => {
    transitionEl.style.opacity = '1';
    setTimeout(() => window.location.reload(), 300);
  }, 10);
};

/**
 * ------------------------------------------------------------------
 *  Lifecycle
 * ------------------------------------------------------------------
 */

onMounted(() => {
  init();

  // 若登录窗口设置 needRefreshHome 标记，则刷新
  if (sessionStorage.getItem('needRefreshHome') === 'true') {
    sessionStorage.removeItem('needRefreshHome');
    refreshEntireWindow();
  }

  const messageHandler = (event: MessageEvent) => {
    if (event.data?.type === 'login-success' && event.data.refreshHome) {
      refreshEntireWindow();
    }
  };
  window.addEventListener('message', messageHandler);

  onUnmounted(() => {
    window.removeEventListener('message', messageHandler);
  });
});
</script>

<template>
  <div>
    <!-- 标题 & 欢迎语 -->
    <n-h2 style="margin-bottom: 3px;">欢迎回来</n-h2>
    欢迎使用全新 <n-gradient-text type="info">OpenFrp Cross Platform Launcher</n-gradient-text> !

    <div style="margin-top: 10px;">
      <n-space vertical>
        <!-- 头部公告 -->
        <div v-if="headAlert?.status" style="margin-bottom: -8px;" v-external>
          <n-alert :title="headAlert.title" :type="headAlert.type" closable style="margin-bottom: 8px;">
            <div v-html="headAlert.content" />
          </n-alert>
        </div>

        <!-- 登录提醒 -->
        <n-card v-if="!isLoggedIn" :bordered="false" style="text-align:center;">
          <n-flex vertical align="center" justify="center" :size="12">
            <n-icon size="64" color="#f0a020"><PersonCircleOutline /></n-icon>
            <n-h4 style="margin:0;">您尚未登录</n-h4>
            <n-text depth="3">登录后可同步配置、管理隧道并解锁更多功能</n-text>
            <n-space :size="16" style="margin-top:8px;">
              <n-button type="primary" @click="goToLogin">立即登录</n-button>
              <n-button @click="openUrl('https://account.naids.com/register')" tertiary>立即注册</n-button>
            </n-space>
          </n-flex>
        </n-card>

        <!-- 用户信息 -->
        <n-card v-else style="height: 100%">
          <n-skeleton v-if="loading" height="250px" :sharp="false" size="medium" />

          <n-flex v-else vertical justify="space-between" style="height: 100%">
            <n-descriptions :column="2">
              <n-descriptions-item label="主权限组">{{ userInfo?.friendlyGroup }}</n-descriptions-item>
              <n-descriptions-item label="隧道数">{{ userInfo?.used }} / {{ userInfo?.proxies }}</n-descriptions-item>
              <n-descriptions-item label="注册邮箱">{{ userInfo?.email }}</n-descriptions-item>
              <n-descriptions-item label="带宽速率 (上 / 下)">
                {{ byteFormat(userInfo?.outLimit || 0).replace('Mb', '') }} Mbps /
                {{ byteFormat(userInfo?.inLimit || 0).replace('Mb', '') }} Mbps
              </n-descriptions-item>
              <n-descriptions-item label="注册时间">{{ userInfo?.regTime }}</n-descriptions-item>
              <n-descriptions-item label="剩余流量">
                {{
                  numbro((userInfo?.traffic || 0) * 1024 * 1024).format({
                    output: 'byte',
                    base: 'binary',
                    mantissa: 2,
                  })
                }}
              </n-descriptions-item>
            </n-descriptions>

            <n-space vertical :size="[0, 8]">
              <n-text depth="3">签到可以获得免费流量，打开网页面板立即完成每日签到。</n-text>

              <n-space v-if="userInfo" :size="[24, 0]">
                <!-- 复制访问密钥 -->
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button
                      type="warning"
                      text
                      strong
                      @click="
                        userInfo?.token && copy(userInfo.token);
                        message.success('已复制至剪贴板, 请妥善保管。');
                      "
                    >
                      <template #icon><n-icon><KeyOutline /></n-icon></template>
                      复制访问密钥
                    </n-button>
                  </template>
                  复制访问密钥
                </n-tooltip>

                <!-- 签到按钮 -->
                <n-tooltip v-if="!signed" trigger="hover">
                  <template #trigger>
                    <n-button type="success" text @click="userSign">
                      <template #icon><n-icon><CalendarOutline /></n-icon></template>
                      签到
                    </n-button>
                  </template>
                  立即签到获取更多流量
                </n-tooltip>
              </n-space>
            </n-space>
          </n-flex>
        </n-card>

        <!-- 系统公告 -->
        <n-card title="系统公告">
          <div class="markdown" v-html="marked.parse(announcement)" v-external style="margin-top: -16px" />
        </n-card>
      </n-space>
    </div>
  </div>
</template>

<style scoped>
.markdown p {
  margin: 0;
}

.markdown hr {
  border-color: #ffffff42;
}

.markdown .actual-dark hr {
  border-color: #3c3c3c;
}

.markdown .right-bottom-bottom {
  display: none !important;
}

.markdown pre {
  margin: 10px 0;
  border-radius: 5px;
  box-shadow: rgba(0, 0, 0, 0.55) 0 2px 10px;
}

.markdown pre::before {
  content: '';
  display: block;
  background: url('https://cdn.futo.design/bop-web/assets/code-bar.svg') 10px 10px/40px no-repeat #282c34;
  height: 30px;
  width: 100%;
  margin-bottom: -7px;
  border-radius: 5px 5px 0 0;
}

.markdown pre code {
  overflow-x: auto;
  padding: 16px 16px 15px;
  color: #abb2bf;
  display: -webkit-box;
  font-family: Operator Mono, Consolas, Monaco, Menlo, monospace;
  font-size: 15px;
  background: #282c34;
  border-radius: 5px;
}

@keyframes spin {
  to {
    transform: translate(-50%, -50%) rotate(360deg);
  }
}
</style>
