// import axios from 'axios';
// import Cookies from 'js-cookie';
import Cookies from '../../utils/cookies';
import { callApi } from '../../utils/apiClient';

interface Response {
  data: Struct.UserInfo;
  flag: boolean;
  msg: string;
}


const clearAuth = () => {
  Cookies.remove('authorization');
  localStorage.removeItem('userToken');
};

export default async () => {
  try {
    const response = await callApi<Response>('getUserInfo', {
      method: 'POST',
      body: {},
    });
    
    // 检查响应是否为 null 或 undefined
    if (!response) {
      throw new Error('API 返回了空响应');
    }
    
    return response;
  } catch (error) {
    console.error('获取用户信息失败:', error);
    // 未登录时清理本地登录态
    if (error instanceof Error && error.message === '未登录') {
      clearAuth();
    }
    // 返回一个默认响应，避免 null 引用错误
    return {
      data: null as unknown as Struct.UserInfo,
      flag: false,
      msg: error instanceof Error ? error.message : '未知错误'
    };
  }
};
