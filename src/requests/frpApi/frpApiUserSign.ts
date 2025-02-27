import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: string;
  flag: boolean;
  msg: string;
}

export default (req: any) => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/userSign',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: req,
  });
};
