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
    const response = await invoke<T>('proxy_api', {
      url: endpoint,
      method,
      headers: authHeaders,
      body,
    });
    
    return response;
  } catch (error) {
    console.error(`API调用失败 (${endpoint}):`, error);
    throw error;
  }
}