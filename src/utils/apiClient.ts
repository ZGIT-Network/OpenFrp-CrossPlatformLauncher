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
    
    // 检查响应头中的 authorization
    if (response.headers && response.headers.authorization) {
      const responseAuth = response.headers.authorization;
      const currentAuth = Cookies.get('authorization');
      
      // 如果响应头中的 authorization 与 cookies 中的不一致，更新 cookies
      if (responseAuth !== currentAuth) {
        console.log('检测到 authorization 更新，正在更新 cookies');
        Cookies.set('authorization', responseAuth);
      }
    }
    
    return response.data;
  } catch (error) {
    console.error(`API调用失败 (${endpoint}):`, error);
    throw error;
  }
}