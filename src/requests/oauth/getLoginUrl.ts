import axios from 'axios';
import { OPENFRP_API_BASES } from '@/utils/apiBase';

interface Response {
  data: string;
  flag: boolean;
  msg: string;
}

export default async (redirectUrl: string) => {
  let lastError: unknown;
  let lastResponse: { data: Response } | undefined;

  for (const base of OPENFRP_API_BASES) {
    try {
      const res = await axios.request<Response>({
        url: `${base}/oauth2/login`,
        method: 'get',
        params: { redirect_url: redirectUrl },
        timeout: 15000,
      });

      if (res.data?.flag) {
        return res;
      }

      // 服务端明确返回失败时记录响应，继续尝试备用地址
      lastResponse = res;
      lastError = new Error(res.data?.msg || '无法获取登录地址');
    } catch (error) {
      lastError = error;
    }
  }

  // 所有地址都返回了业务错误时，把最后一次响应交给调用方展示
  if (lastResponse) return lastResponse;
  throw lastError instanceof Error ? lastError : new Error(String(lastError));
};
