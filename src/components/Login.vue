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
          
          // 使用平滑过渡而不是直接刷新
          // 创建一个过渡元素
          const transitionEl = document.createElement('div');
          transitionEl.style.position = 'fixed';
          transitionEl.style.top = '0';
          transitionEl.style.left = '0';
          transitionEl.style.width = '100%';
          transitionEl.style.height = '100%';
          transitionEl.style.backgroundColor = '#121212'; // 深色背景
          transitionEl.style.zIndex = '9999';
          transitionEl.style.opacity = '0';
          transitionEl.style.transition = 'opacity 0.3s ease';
          
          // 添加加载动画
          const spinner = document.createElement('div');
          spinner.style.width = '40px';
          spinner.style.height = '40px';
          spinner.style.border = '4px solid rgba(255, 255, 255, 0.1)';
          spinner.style.borderRadius = '50%';
          spinner.style.borderTopColor = '#18a058';
          spinner.style.position = 'absolute';
          spinner.style.top = '50%';
          spinner.style.left = '50%';
          spinner.style.transform = 'translate(-50%, -50%)';
          spinner.style.animation = 'spin 1s linear infinite';
          
          // 添加动画关键帧
          const style = document.createElement('style');
          style.textContent = '@keyframes spin { to { transform: translate(-50%, -50%) rotate(360deg); } }';
          document.head.appendChild(style);
          
          transitionEl.appendChild(spinner);
          document.body.appendChild(transitionEl);
          
          // 淡入过渡元素
          setTimeout(() => {
            transitionEl.style.opacity = '1';
            
            // 淡入后再跳转并刷新
            setTimeout(() => {
              router.push(redirectPath).then(() => {
                window.location.reload();
              });
            }, 300);
          }, 10);
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
