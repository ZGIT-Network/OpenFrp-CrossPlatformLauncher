import axios from 'axios';
import Cookies from 'js-cookie';

interface ResponseData {
  list: Struct.UserProxy[];
  total: number;
}

interface Response {
  data: ResponseData;
  flag: boolean;
  msg: string;
}

export default () => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/getUserProxies',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {},
  });
};
