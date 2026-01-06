import { invoke } from '@tauri-apps/api/core';
import Cookies from '@/utils/cookies';

interface ApiOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
  headers?: Record<string, string>;
  body?: any;
}

// 添加登录状态检查
export function isLoggedIn(): boolean {
  return !!Cookies.get('authorization');
}

interface ApiResponse<T> {
  data: T;
  headers: Record<string, string>;
}

export async function callApi<T>(endpoint: string, options: ApiOptions = {}): Promise<T> {
  // 自动处理登录失效的统一逻辑
  const handleAuthInvalid = (msg?: string) => {
    // 判断信息是否包含未登录、登录失效等关键词
    const invalid = msg && /未登录|登录失效|token 失效|凭证失效|unauthorized/i.test(msg);
    if (invalid) {
      console.warn('检测到登录凭证失效，正在清理本地登录状态');
      Cookies.remove('authorization');
      localStorage.removeItem('userToken');
      // 触发一次简单的页面刷新或路由跳转，以便 UI 及时更新
      try {
        if (typeof window !== 'undefined') {
          // 避免在子窗口或无路由环境下报错
          window.dispatchEvent(new Event('auth-invalid'));
        }
      } catch { /* ignore */ }

    }
  };
  // 检查是否登录
  if (!isLoggedIn()) {
    throw new Error('未登录');
  }

  const { method = 'GET', headers = {}, body } = options;
  
  const authHeaders = {
    ...headers,
    Authorization: Cookies.get('authorization') || '',
  };
  
  try {
    const response = await invoke<ApiResponse<T>>('proxy_api', {
      url: endpoint,
      method,
      headers: authHeaders,
      body,
    });
    
    // 检查响应头中的 authorization（忽略大小写）
    if (response.headers) {
      const authHeaderKey = Object.keys(response.headers).find(k => k.toLowerCase() === 'authorization');
      if (authHeaderKey) {
        const responseAuth = (response.headers as any)[authHeaderKey];
        const currentAuth = Cookies.get('authorization');
        // 如果响应头中的 authorization 与 cookies 中的不一致，更新 cookies
        if (responseAuth && responseAuth !== currentAuth) {
          console.log('检测到 authorization 更新，正在更新 cookies');
          Cookies.set('authorization', responseAuth);
        }
      }
    }
    
    // 如果接口返回 flag=false 表示失败，且可能是登录失效
    if (response.data && typeof response.data === 'object' && 'flag' in response.data) {
      const anyResp: any = response.data;
      if (anyResp.flag === false) {
        handleAuthInvalid(anyResp.msg);
      }
    }

    return response.data;
  } catch (error: any) {
    // 捕获到异常时，也检查是否因登录失效导致
    handleAuthInvalid(error?.message);

    console.error(`API调用失败 (${endpoint}):`, error);
    throw error;
  }
}