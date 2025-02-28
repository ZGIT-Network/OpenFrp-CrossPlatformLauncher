import { callApi } from '../../utils/apiClient';

interface ResponseData {
  list: Struct.UserProxy[];
  total: number;
}

interface Response {
  data: ResponseData;
  flag: boolean;
  msg: string;
}

export default async () => {
  try {
    const response = await callApi<Response>('getUserProxies', {
      method: 'POST',
      body: {},
    });
    
    // 检查响应是否为 null 或 undefined
    if (!response) {
      throw new Error('API 返回了空响应');
    }
    
    return response;
  } catch (error) {
    console.error('获取用户隧道列表失败:', error);
    // 返回一个默认响应，避免 null 引用错误
    return {
      data: { list: [], total: 0 } as ResponseData,
      flag: false,
      msg: error instanceof Error ? error.message : '未知错误'
    };
  }
};