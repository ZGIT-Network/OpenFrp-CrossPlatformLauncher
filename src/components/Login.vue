<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue';
import { useLoadingBar, useMessage } from 'naive-ui';
import { useRoute, useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';

const message = useMessage();
const router = useRouter();
const route = useRoute();
const loadingBar = useLoadingBar();
const session = useSessionStore();
const code = ref('');
const pending = ref(false);
const failure = ref('');
let active = true;

watch(() => route.query.code, (value) => {
  if (typeof value === 'string') {
    code.value = value;
    void router.replace({ query: {} });
  }
}, { immediate: true });
onUnmounted(() => {
  active = false;
  if (pending.value) loadingBar.finish();
});

async function completeLogin() {
  if (!code.value || pending.value || session.signedIn) return;
  const callbackCode = code.value;
  code.value = '';
  pending.value = true;
  loadingBar.start();
  try {
    const result = await invoke<{ authorization: string; msg: string }>('oauth_callback', { code: callbackCode });
    if (!active) return;
    await session.signIn(result.authorization, () => active);
    if (!active) return;
    message.success(result.msg || '登录成功');
    loadingBar.finish();
    await router.replace('/home');
  } catch (error) {
    if (active) {
      failure.value = errorMessage(error, '登录失败，请重新发起登录');
      loadingBar.error();
    }
  } finally {
    pending.value = false;
  }
}
</script>

<template>
  <n-result v-if="pending" status="info" title="登录处理中" description="正在验证您的身份，请稍候..." class="login-result">
    <template #footer><n-spin size="large" /></template>
  </n-result>
  <n-result v-else-if="failure" status="error" title="登录失败" :description="failure" class="login-result">
    <template #footer><n-button @click="router.replace('/settings')">返回设置</n-button></template>
  </n-result>
  <n-result v-else-if="code && !session.signedIn" status="info" title="确认登录" description="确认使用收到的授权链接登录启动器。" class="login-result">
    <template #footer>
      <n-space justify="center">
        <n-button @click="router.replace('/settings')">取消</n-button>
        <n-button type="primary" @click="completeLogin">继续登录</n-button>
      </n-space>
    </template>
  </n-result>
  <n-result v-else status="info" title="欢迎使用" description="请在设置中完成登录" class="login-result">
    <template #footer><n-button @click="router.replace('/settings')">打开设置</n-button></template>
  </n-result>
</template>

<style scoped>
.login-result {
  max-width: 600px;
  padding: 10vh 12px 24px;
  margin: auto;
}
</style>
