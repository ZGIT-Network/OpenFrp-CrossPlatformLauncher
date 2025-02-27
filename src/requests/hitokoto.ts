import axios from 'axios';

export default () => {
  return axios.request<Response>({
    url: 'https://v1.hitokoto.cn/?c=i&encode=json',
    method: 'post',
  });
};
