import { invoke } from '@tauri-apps/api/core';

type OAuthResponse = {
  authorization: string;
  flag: boolean;
  msg: string;
  data: string;
};

export default async (code: string, redirectUrl?: string) => {
  try {
    const resp = (await invoke('oauth_callback', { code, redirectUrl })) as OAuthResponse;
    return {
      data: resp,
      headers: {
        authorization: resp.authorization
      }
    };
  } catch (error: any) {
    console.error('OAuth 请求失败:', error);
    // Tauri 命令返回的 Err(String) 会被包装成字符串，统一转换成 Error 便于上层展示
    throw new Error(typeof error === 'string' ? error : (error?.message || 'OAuth 请求失败'));
  }
};