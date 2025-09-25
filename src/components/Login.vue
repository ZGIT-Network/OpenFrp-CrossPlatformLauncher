<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import Cookies from '@/utils/cookies';
// 修复组件导入，确保从naive-ui中正确导入所需组件
import { useLoadingBar, useMessage } from 'naive-ui';
import { NResult, NSpin } from 'naive-ui';
import oauthCallback from '@/requests/oauth/oauthCallback';

const message = useMessage();
const router = useRouter();
const route = useRoute();
const loadingbar = useLoadingBar();
const loginErr = ref<string | undefined>(undefined);
const isProcessing = ref(false);
const loginStatus = ref<'processing' | 'success' | 'error' | 'idle'>('idle');

// 处理登录逻辑
onMounted(() => {
  // 如果已经登录，直接跳转到首页
  if (Cookies.get('authorization')) {
    console.log('已登录，跳转到首页');
    router.push('/home');
    return;
  }

  // 开始处理OAuth回调
  loadingbar.start();
  const callbackCode = route.query.code;
  
  // 清除URL中的查询参数
  router.replace({ query: {} });
  
  if (callbackCode) {
    isProcessing.value = true;
    loginStatus.value = 'processing';
    
    oauthCallback(String(callbackCode))
      .then((res) => {
        if (res.data.flag) {
          const authorization = res.headers.authorization;
          if (!authorization) throw new Error('未获取到授权信息');
          Cookies.set('authorization', authorization, { expires: 7 });
          message.success(res.data.msg || '登录成功');
          loginStatus.value = 'success';
          
          // 处理重定向
          const redirectPath = sessionStorage.getItem('redirectPath') || '/home';
          sessionStorage.removeItem('redirectPath');
          
          // 使用平滑过渡
          setTimeout(() => {
            router.push(redirectPath).then(() => {
              router.go(0);
              // window.location.reload();
              // window.dispatchEvent(new Event('userInfoUpdated'));
              console.log('登录成功，跳转到:', redirectPath);
              loadingbar.finish();
            });
          }, 1000);
        } else {
          const errorMsg = (res.data.data as any)?.msg || '登录失败';
          loginErr.value = errorMsg;
          message.error(errorMsg);
          loadingbar.error();
        }
      })
      .catch((error) => {
        console.error('登录过程出错:', error);
        loginErr.value = error.message || '登录过程出错';
        message.error(error.message || '登录过程出错');
        loadingbar.error();
        loginStatus.value = 'error';
      })
      .finally(() => {
        isProcessing.value = false;
        loadingbar.finish();
      });
  } else {
    // 没有回调代码，可能是直接访问登录页
    setTimeout(() => {
      loadingbar.finish();
    }, 1000);
  }
});
</script>

<template>
  <div class="login-container">
    <n-result
      v-if="loginStatus === 'processing'"
      status="info"
      title="登录处理中"
      description="正在验证您的身份，请稍候..."
    >
      <template #footer>
        <n-spin size="large" />
      </template>
    </n-result>

    <n-result
      v-else-if="loginStatus === 'error'"
      status="error"
      title="登录失败"
      :description="loginErr || '登录过程中发生错误'"
    />

    <n-result
      v-else-if="loginStatus === 'success'"
      status="success"
      title="登录成功"
      description="正在跳转到系统..."
    >
      <template #footer>
        <n-spin size="small" />
      </template>
    </n-result>

    <n-result
      v-else
      status="info"
      title="欢迎使用"
      description="请通过授权链接登录系统"
    />
  </div>
</template>

<style scoped>
.login-container {
  margin-top: 15vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  max-width: 600px;
  margin-left: auto;
  margin-right: auto;
}
</style>
