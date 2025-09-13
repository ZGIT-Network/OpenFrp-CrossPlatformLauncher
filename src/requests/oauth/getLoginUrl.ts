import axios from 'axios';

interface Response {
  data: string;
  flag: boolean;
  msg: string;
}

export default () => {
  return axios.request<Response>({
    url: 'https://of-dev-api.bfsea.com/oauth2/login?redirect_url=https://staticassets.naids.com/ofcpl_login',
    method: 'get',
  });
};
