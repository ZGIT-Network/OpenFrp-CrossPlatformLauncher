import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default (req: bigint) => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/refreshProxyStatus',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {
      ...{
        proxy_id: req,
      },
    },
  });
};
