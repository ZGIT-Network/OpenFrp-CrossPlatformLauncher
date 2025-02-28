import { callApi } from '../../utils/apiClient';

interface SignRequest {
  ticket: string;
  randstr?: string;
}

interface Response {
  data: string;
  flag: boolean;
  msg: string;
}

export default async (req: SignRequest) => {
  try {
    // 确保请求参数格式正确
    const response = await callApi<Response>('userSign', {
      method: 'POST',
      body: req,
    });
    
    // 检查响应是否为 null 或 undefined
    if (!response) {
      throw new Error('API 返回了空响应');
    }
    
    return response;
  } catch (error) {
    console.error('用户签到失败:', error);
    // 返回一个默认响应，避免 null 引用错误
    return {
      data: '',
      flag: false,
      msg: error instanceof Error ? error.message : '签到失败，请稍后重试'
    };
  }
};