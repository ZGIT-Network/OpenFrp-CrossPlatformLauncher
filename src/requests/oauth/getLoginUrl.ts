import axios from 'axios';

interface Response {
  data: string;
  flag: boolean;
  msg: string;
}

export default () => {
  return axios.request<Response>({
    url: 'https://api.openfrp.net/oauth2/login?redirect_url=openfrp://login',
    method: 'get',
  });
};
