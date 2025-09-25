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
  } catch (error) {
    console.error('OAuth 请求失败:', error);
    throw error;
  }
};