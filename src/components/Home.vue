<script setup lang="ts">
import { inject, Ref, ref, watch, onMounted, onUnmounted, computed } from 'vue';
import { NAlert, NGradientText, useMessage, NCard, NSpace, useNotification } from 'naive-ui'
import axios from 'axios'
import { marked } from 'marked'
import copy from 'copy-to-clipboard';
import dayjs from 'dayjs';
import { useRouter } from 'vue-router';

import  Coockies  from '@/utils/cookies';

import { openUrl } from '@tauri-apps/plugin-opener';

// 导入获取用户信息的API
import frpApiGetUserInfo from '@/requests/frpApi/frpApiGetUserInfo';

import { CalendarOutline, KeyOutline } from '@vicons/ionicons5';

import numbro from 'numbro';

const message = useMessage()

import frpApiGetSignInfo from '@/requests/frpApi/frpApiGetSignInfo';

const signInfo = ref<Struct.SignInfo>();
const signed = ref<boolean>();
const loading = ref(true); // 添加一个加载状态变量

const router = useRouter();

// 尝试注入对象
const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');
console.log('注入的对象:', userInfoObj);

const byteFormat = (num: number) => {
  return numbro(num * 1024 * 8)
    .format({ output: 'byte', base: 'general' })
    .replace('B', 'b');
};

// 创建本地用户名变量
const username = ref('');
const userInfo = userInfoObj?.userInfo;



const notification = useNotification();

// 监听用户信息变化
if (userInfoObj?.userInfo) {
  watch(userInfoObj.userInfo, (newVal) => {
    if (newVal) {
      username.value = newVal.username || '';
      console.log('用户名已更新:', username.value);
      loading.value = false; // 用户信息加载完成
    }
  }, { immediate: true });
}

const boardCast = ref('加载中...');
const headAlert = ref();

const fetchBoardCast = async () => {
  try {
    const res = await axios.get(`https://api.openfrp.net/commonQuery/get`, {
      params: {
        key: 'help_info',
      }
    })

    headAlert.value = (res.data.data as any).headalert;
    boardCast.value = (res.data.data as any).cpl_announce;
    // console.log(boardCast.value) 

  } catch (e) {
    message.error(`获取公告失败: ${e}`)
  }
}

// 修改获取签到信息的函数，添加重试机制
const getSignInfo = () => {
  frpApiGetSignInfo().then((res) => {
    if (res.flag) {
      signInfo.value = res.data;
      signed.value = dayjs(res.data.signdate).diff(dayjs().startOf('day')) >= 0;
    } else {
      message.error(res.msg);
    }
  });
};

// 添加一个直接获取用户信息的函数，以防注入失败
// 修改获取用户信息的函数
const fetchUserInfo = (retries = 2) => {
  if (!isLoggedIn.value) {
    loading.value = false;
    return;
  }

  if (userInfo?.value) {
    loading.value = false;
    return;
  }
  
  console.log('获取用户信息');
  frpApiGetUserInfo().then((res) => {
    if (res.flag) {
      if (userInfoObj && !userInfoObj.userInfo.value) {
        userInfoObj.userInfo.value = res.data;
      }
      loading.value = false;
    } else {
      if (retries > 0) {
        setTimeout(() => {
          fetchUserInfo(retries - 1);
        }, 1000); // 增加重试间隔
      } else {
        loading.value = false;
        message.error('获取用户信息失败');
      }
    }
  }).catch(() => {
    loading.value = false;
  });
};

// 修改检查函数
const checkinfo = () => {
  if (!isLoggedIn.value) {  // 修改这里，使用 .value 访问计算属性的值
    loading.value = false;
    return;
  }

  if(userInfo?.value) {
    getSignInfo();
    getSignInfo();
    checkRealname(); 
  } else {
    fetchUserInfo();
    getSignInfo();

  }
}
const checkRealname = () => {
  if (!userInfo?.value?.realname) {
    message.warning('您尚未实名认证，请尽快进行实名认证以体验最佳服务');
    notification.error({
      title: '您尚未完成实名认证',
      content: '请尽快进行实名认证以获得最佳服务体验。未实名将无法使用中国大陆地区节点。\n',
      keepAliveOnHover: true,
      closable: true,
    });
  }
};

const isLoggedIn = computed(() => {
  const loggedIn = !!userInfo?.value?.token;
  console.log('登录状态:', loggedIn, '用户信息:', userInfo?.value);
  return loggedIn;
});

// 跳转到设置页面进行登录
const goToLogin = () => {
  router.push('/settings');
  message.info('请在设置页面完成登录');
};

// const checkinfo = () => {
//   if(isLoggedIn.value) {
//     getSignInfo();
//     checkRealname(); 
//   } else {
//     // 如果没有登录状态，但有可能是因为注入失败，尝试直接获取用户信息
//     fetchUserInfo();
//   }
// }

// 添加一个超时检查，如果长时间加载不出来，提示用户刷新
onMounted(() => {
  setTimeout(() => {
    if (loading.value && !userInfo?.value) {
      message.warning('加载用户信息超时，请尝试刷新页面');
    }
  }, 5000);
  
  checkinfo();
  fetchBoardCast();
});

function userSign() {
  try {
      const auth = Coockies.get('authorization');
      if (auth) {
        openUrl('https://console.openfrp.net/fastlogin?auth=' + auth+'&type=sign');
      } else {
        openUrl('https://console.openfrp.net/?type=sign');
      }
    } catch (error) {
      console.error('打开网页面板失败:', error);
    }
};


// 添加刷新整个窗体的函数
const refreshEntireWindow = () => {
  console.log('刷新整个窗体');
  
  // 创建一个过渡元素
  const transitionEl = document.createElement('div');
  transitionEl.style.position = 'fixed';
  transitionEl.style.top = '0';
  transitionEl.style.left = '0';
  transitionEl.style.width = '100%';
  transitionEl.style.height = '100%';
  transitionEl.style.backgroundColor = '#121212'; // 深色背景
  transitionEl.style.zIndex = '9999';
  transitionEl.style.opacity = '0';
  transitionEl.style.transition = 'opacity 0.3s ease';
  
  // 添加加载动画
  const spinner = document.createElement('div');
  spinner.style.width = '40px';
  spinner.style.height = '40px';
  spinner.style.border = '4px solid rgba(255, 255, 255, 0.1)';
  spinner.style.borderRadius = '50%';
  spinner.style.borderTopColor = '#18a058';
  spinner.style.position = 'absolute';
  spinner.style.top = '50%';
  spinner.style.left = '50%';
  spinner.style.transform = 'translate(-50%, -50%)';
  spinner.style.animation = 'spin 1s linear infinite';
  
  // 添加动画关键帧
  const style = document.createElement('style');
  style.textContent = '@keyframes spin { to { transform: translate(-50%, -50%) rotate(360deg); } }';
  document.head.appendChild(style);
  
  transitionEl.appendChild(spinner);
  document.body.appendChild(transitionEl);
  
  // 淡入过渡元素
  setTimeout(() => {
    transitionEl.style.opacity = '1';
    
    // 淡入后再刷新页面
    setTimeout(() => {
      window.location.reload();
    }, 300);
  }, 10);
};
onMounted(() => {
  // 检查是否需要刷新页面
  if (sessionStorage.getItem('needRefreshHome') === 'true') {
    // 清除标记
    sessionStorage.removeItem('needRefreshHome');
    // 刷新整个窗体
    refreshEntireWindow();
  }

  // 监听来自登录窗口的消息
  const messageHandler = (event: MessageEvent) => {
    if (event.data && event.data.type === 'login-success' && event.data.refreshHome) {
      // 刷新整个窗体
      refreshEntireWindow();
    }
  };

  window.addEventListener('message', messageHandler);

  // 组件卸载时移除事件监听
  onUnmounted(() => {
    window.removeEventListener('message', messageHandler);
  });
});
</script>

<template>
  <div>
    <!-- 使用本地用户名变量 -->
    <N-H2 style="margin-bottom: 3px;">欢迎回来</N-H2>
    欢迎使用全新 <n-gradient-text type="info">
      OpenFrp Cross Platform Launcher
    </n-gradient-text> !
    <div style="margin-top: 10px;">
      <n-space vertical>
        <!-- <n-alert type="warning">您当前正在使用 Beta 测试版本，可能存在一些问题，请谨慎在生产环境使用。<br />若遇到问题，请及时与开发则反馈。</n-alert> -->

        <div v-if="headAlert?.status" style="margin-bottom: -8px;" v-external>
          <n-alert :title="headAlert.title" :type="headAlert.type" closable style="margin-bottom: 8px;">
            <div v-html="headAlert.content"></div>
          </n-alert>
        </div>
        <n-card v-if="!isLoggedIn">
          <n-result status="warning" title="您尚未登录" description="请先登录以使用完整功能">
            <template #footer>
              <n-button type="primary" @click="goToLogin">
                前往登录
              </n-button>
            </template>
          </n-result>
        </n-card>

        <n-card v-else style="height: 100%">
          <!-- 修改骨架屏显示条件，使用loading状态 -->
          <n-skeleton v-if="loading" style="margin-top: 0.75rem" height="250px"
            width="100%" :sharp="false" size="medium" />
          <n-flex v-else style="height: 100%" justify="space-between" vertical>
            <n-descriptions :column="2">
              <n-descriptions-item label="主权限组">
                {{ userInfo?.friendlyGroup }}
              </n-descriptions-item>

              <n-descriptions-item label="隧道数">{{ userInfo?.used }} / {{ userInfo?.proxies }}
              </n-descriptions-item>
              <n-descriptions-item label="注册邮箱">
                <span>{{ userInfo?.email }}</span>
              </n-descriptions-item>
              <n-descriptions-item label="带宽速率 (上 / 下)">
                <span>{{ byteFormat(userInfo?.outLimit || 0).replace('Mb', '') }} Mbps</span>
                /
                <span>{{ byteFormat(userInfo?.inLimit || 0).replace('Mb', '') }} Mbps</span>
              </n-descriptions-item>

              <n-descriptions-item label="注册时间">
                {{ userInfo?.regTime }}
                <!-- {{ dayjs((userInfo?.regTime || 0) * 1000).format('YYYY-MM-DD') }} -->
              </n-descriptions-item>
              <n-descriptions-item label="剩余流量">{{
                numbro((userInfo?.traffic || 0) * 1024 * 1024).format({
                  output: 'byte',
                  base: 'binary',
                  mantissa: 2,
                })
              }}
              </n-descriptions-item>
            </n-descriptions>
            <n-space vertical :size="[0, 8]">
              <n-text :depth="3">签到可以获得免费流量，打开网页面板完成立即完成每日签到。
              </n-text>
              <!-- 修改这里的条件判断 -->
              <n-space v-if="userInfo != null" :size="[24, 0]">
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button type="warning" text strong @click="
                      userInfo?.token && copy(userInfo.token);
                      message.success('已复制至剪贴板,请妥善保管。');
                    ">
                      <template #icon>
                        <n-icon>
                          <KeyOutline />
                        </n-icon>
                      </template>
                      复制访问密钥
                    </n-button>
                  </template>
                  复制访问密钥
                </n-tooltip>
                <n-tooltip v-if="!signed" trigger="hover">
                  <template #trigger>
                    <n-button type="success" text @click="userSign">
                      <template #icon>
                        <n-icon>
                          <CalendarOutline />
                        </n-icon>
                      </template>
                      签到
                    </n-button>
                  </template>
                 立即签到获取更多流量
                </n-tooltip>
              </n-space>
            </n-space>
          </n-flex>
        </n-card>

        <n-card title="系统公告">

          <div class="markdown" style="margin-top: -16px" v-html="marked.parse(boardCast)" v-external ></div>
        </n-card>
       
      </n-space>
    </div>
  </div>
</template>

<style scoped>
.hometitle {
  font-size: 25px;
}

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
  margin-top: 10px;
  margin-bottom: 10px;
  border-radius: 5px;
  box-shadow: rgba(0, 0, 0, 0.55) 0px 2px 10px;
}

.markdown pre::before {
  content: '';
  display: block;
  background: url('https://cdn.futo.design/bop-web/assets/code-bar.svg') 10px 10px / 40px no-repeat rgb(40, 44, 52);
  height: 30px;
  width: 100%;
  margin-bottom: -7px;
  border-radius: 5px 5px 0 0;
}

.markdown pre code {
  overflow-x: auto;
  padding: 16px;
  color: #abb2bf;
  display: -webkit-box;
  font-family: Operator Mono, Consolas, Monaco, Menlo, monospace;
  font-size: 15px;
  -webkit-overflow-scrolling: touch;
  padding-top: 15px;
  background: #282c34;
  border-radius: 5px;
}
</style>
