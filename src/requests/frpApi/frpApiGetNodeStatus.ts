import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default () => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/getNodeStatus',
    method: 'get',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
  });
};
