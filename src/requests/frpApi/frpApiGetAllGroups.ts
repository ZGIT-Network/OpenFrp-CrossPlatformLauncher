import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: Struct.UserAllGroup[];
  flag: boolean;
  msg: string;
}

export default () => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/getAllGroups',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: {},
  });
};
