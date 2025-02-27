import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default (size: number) => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/getTrafficHistory',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {
      history_size: size > 0 ? size : 7,
    },
  });
};
