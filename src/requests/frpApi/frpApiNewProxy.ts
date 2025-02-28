import { callApi } from '../../utils/apiClient';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default async (req: Struct.EditOrNewUserProxy) => {
  req.proxy_id = undefined;
  try {
    const response = await callApi<Response>('newProxy', {
      method: 'POST',
      body: req,
    });
    
    // 检查响应是否为 null 或 undefined
    if (!response) {
      throw new Error('API 返回了空响应');
    }
    
    return response;
  } catch (error) {
    console.error('创建新隧道失败:', error);
    // 返回一个默认响应，避免 null 引用错误
    return {
      data: null,
      flag: false,
      msg: error instanceof Error ? error.message : '未知错误'
    };
  }
};