import { invoke } from '@tauri-apps/api/core';
import { readonly, ref } from 'vue';
import Cookies from '@/utils/cookies';

interface ApiOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
  headers?: Record<string, string>;
  body?: unknown;
}

interface ApiResponse<T> {
  data: T;
  headers: Record<string, string>;
  status: number;
}

const credential = ref(Cookies.get('authorization')?.trim() || '');
export const authorization = readonly(credential);
let sessionVersion = 0;

export const getSessionVersion = () => sessionVersion;

export function setAuthorization(value: string) {
  const token = value.trim();
  sessionVersion++;
  credential.value = token;
  if (token) {
    Cookies.set('authorization', token, { expires: 7 });
  } else {
    Cookies.remove('authorization');
  }
  // 旧版本把 Authorization 同时写进了访问密钥字段。
  localStorage.removeItem('userToken');
}

export function isLoggedIn(): boolean {
  return !!credential.value;
}

export function errorMessage(error: unknown, fallback = '未知错误'): string {
  return typeof error === 'string' ? error : error instanceof Error ? error.message : fallback;
}

export async function requestApi<T>(
  endpoint: string,
  auth: string,
  { method = 'GET', headers = {}, body }: ApiOptions = {}
): Promise<ApiResponse<T>> {
  try {
    return await invoke<ApiResponse<T>>('proxy_api', {
      url: endpoint, method, headers: { ...headers, Authorization: auth }, body,
    });
  } catch (error) {
    throw new Error(errorMessage(error, '请求 OpenFrp API 失败'));
  }
}

export async function callApi<T>(endpoint: string, options: ApiOptions = {}): Promise<T> {
  const auth = credential.value;
  const version = sessionVersion;
  if (!auth) throw new Error('未登录');
  const response = await requestApi<T>(endpoint, auth, options);
  if (version !== sessionVersion) throw new Error('登录状态已变化');

  const result = response.data as { flag?: boolean; msg?: unknown } | null;
  const invalid = response.status === 401 || (result?.flag === false && typeof result.msg === 'string'
    && /未登录|未登入|登录失效|登录过期|登入凭证|凭证无效|token\s*失效|凭证失效|unauthorized/i.test(result.msg));
  if (credential.value === auth) {
    if (invalid) {
      setAuthorization('');
    } else if (result?.flag === true && response.headers.authorization?.trim()) {
      const refreshed = response.headers.authorization.trim();
      if (refreshed !== auth) {
        credential.value = refreshed;
        Cookies.set('authorization', refreshed, { expires: 7 });
      }
    }
  }
  return response.data;
}