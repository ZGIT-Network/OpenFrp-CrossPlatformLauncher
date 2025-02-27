import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

export default (req: Struct.EditOrNewUserProxy) => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/editProxy',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {
      ...req,
    },
  });
};
