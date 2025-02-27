import { invoke } from '@tauri-apps/api/core';
import Cookies from '@/utils/cookies';

interface ApiOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
  headers?: Record<string, string>;
  body?: any;
}

export async function callApi<T>(endpoint: string, options: ApiOptions = {}): Promise<T> {
  const { method = 'GET', headers = {}, body } = options;
  
  // 添加授权头
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
    
    console.log(`API 响应 (${endpoint}):`, response);
    
    // 检查响应是否为 null
    if (response === null || response === undefined) {
      console.error(`API 返回了空响应: ${endpoint}`);
    }
    
    return response;
  } catch (error) {
    console.error(`API调用失败 (${endpoint}):`, error);
    throw error;
  }
}