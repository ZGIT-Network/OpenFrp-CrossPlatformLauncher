<script setup lang="ts">
import { ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

// 添加类型声明以解决 TypeScript 报错
import Cookies from '@/utils/cookies';

import { useLoadingBar, useMessage } from 'naive-ui';

import oauthCallback from '@/requests/oauth/oauthCallback';

const message = useMessage();
const router = useRouter();
const route = useRoute();
const loadingbar = useLoadingBar();
const loginErr = ref<string | undefined>(undefined);
const callbackCode = ref<any>();

if (Cookies.get('authorization')) {
  message.success('已自动登录')
  console.log(Cookies.get('authorization'))
  router.push('/home');
} else {
  loadingbar.start();
  callbackCode.value = route.query.code; // 先copy一份
  router.replace({ query: {} }); // 清除query
  if (callbackCode.value) {
    oauthCallback(String(callbackCode.value))
      .then((res) => {
        // console.log(res)
        if (res.data.flag) {
            const auth = (res.data as any).data['authorization'] || 
                  (res.data as any)?.data.authorization;
          Cookies.set('authorization', (res.data.data as any)?.authorization , {
            expires: 7,
          });
          message.success((res.data as any).data?.msg);
          console.log(auth)
          const redirectPath = '/home'; // 默认登录跳转
          sessionStorage.removeItem('redirectPath');
          window.location.reload();
          router.push(redirectPath);

        } else {
          loginErr.value = (res.data as any).data.msg;
          message.error((res.data as any).data.msg);
          loadingbar.error();
        }
      })
      .catch((x) => message.error(x))
      .finally(() => {
        loadingbar.finish();
      });
  } else {
    setTimeout(() => {
      loadingbar.finish();
      
    }, 1000);
  }
}
</script>
<template>
  <div style="margin-top: 15vh">
  </div>
</template>
