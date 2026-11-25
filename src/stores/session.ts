import { computed, ref, watch } from 'vue';
import { defineStore } from 'pinia';
import {
  authorization, callApi, errorMessage, getSessionVersion, requestApi, setAuthorization,
} from '@/utils/apiClient';

interface UserResponse {
  data?: Struct.UserInfo;
  flag: boolean;
  msg?: string;
}

export const useSessionStore = defineStore('session', () => {
  const userInfo = ref<Struct.UserInfo>();
  const loading = ref(false);
  const profileError = ref('');
  const signedIn = computed(() => !!authorization.value);
  let pending: Promise<void> | undefined;

  watch(signedIn, (active) => {
    if (!active) {
      userInfo.value = undefined;
      profileError.value = '';
      loading.value = false;
      pending = undefined;
    }
  }, { flush: 'sync' });

  function refreshProfile(): Promise<void> {
    if (!signedIn.value) return Promise.resolve();
    if (pending) return pending;
    const version = getSessionVersion();
    loading.value = true;
    profileError.value = '';
    pending = callApi<UserResponse>('getUserInfo', { method: 'POST', body: {} })
      .then((result) => {
        if (version !== getSessionVersion()) return;
        if (!result.flag || !result.data) throw new Error(result.msg || '获取用户信息失败');
        userInfo.value = result.data;
      })
      .catch((error) => {
        if (version === getSessionVersion()) profileError.value = errorMessage(error);
      })
      .finally(() => {
        if (version === getSessionVersion()) {
          loading.value = false;
          pending = undefined;
        }
      });
    return pending;
  }

  async function signIn(value: string, isCurrent = () => true) {
    const version = getSessionVersion();
    const credential = value.trim();
    if (!credential) throw new Error('请输入 Authorization');
    const result = await requestApi<UserResponse>('getUserInfo', credential, { method: 'POST', body: {} });
    if (!result.data.flag || !result.data.data) {
      throw new Error(result.data.msg || '登录失败：无效的 Authorization');
    }
    if (version !== getSessionVersion() || !isCurrent()) {
      throw new Error('本次登录已取消');
    }
    setAuthorization(result.headers.authorization?.trim() || credential);
    userInfo.value = result.data.data;
    loading.value = false;
    pending = undefined;
    profileError.value = '';
  }

  async function signOut() {
    const credential = authorization.value;
    setAuthorization('');
    if (credential) {
      const response = await requestApi<{ flag: boolean; msg?: string }>(
        'logout', credential, { method: 'POST', body: {} }
      );
      if (!response.data.flag && response.status !== 401) {
        throw new Error(response.data.msg || '服务端退出登录失败');
      }
    }
  }

  return { userInfo, signedIn, loading, profileError, refreshProfile, signIn, signOut };
});
