import axios from 'axios';
import Cookies from 'js-cookie';

interface Response {
  data: any;
  flag: boolean;
  msg: string;
}

function logoutCurr() {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/logout',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
  });
}

function logoutAll() {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/frp/api/logoutAll',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
  });
}

function argoAccept(data: any) {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/ext/argoAccept',
    method: 'post',
    headers: {
      Authorization: Cookies.get('authorization') || '',
    },
    data: data,
  });
}

export { logoutCurr, logoutAll, argoAccept };
