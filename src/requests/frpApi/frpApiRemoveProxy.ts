import axios from 'axios';
import Cookies from 'js-cookie';

interface Request {
  proxy_id: bigint;
}

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default (req: Request) => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/removeProxy',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {
      ...req,
    },
  });
};
