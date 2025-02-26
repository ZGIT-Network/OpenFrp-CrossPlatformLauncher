import { invoke } from '@tauri-apps/api/core';

export default async (code: string) => {
  try {
    const auth = await invoke('oauth_callback', { code });
    return {
      data: {
        flag: true,
        msg: '登录成功',
        data: auth
      },
      headers: {
        authorization: auth
      }
    };
  } catch (error) {
    console.error('OAuth 请求失败:', error);
    throw error;
  }
};