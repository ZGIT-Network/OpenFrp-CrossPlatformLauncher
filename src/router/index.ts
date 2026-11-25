import { createRouter, createWebHashHistory } from 'vue-router';

export default createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/home' },
    { path: '/home', name: 'Home', component: () => import('@/components/Home.vue') },
    { path: '/createproxy', name: 'CreateProxy', component: () => import('@/components/CreateProxies/index.vue') },
    { path: '/proxylist', name: 'ProxyList', component: () => import('@/components/ProxyList.vue') },
    { path: '/login', name: 'Login', component: () => import('@/components/Login.vue') },
    { path: '/frpc', name: 'FrpcManager', component: () => import('@/components/FrpcManager.vue') },
    { path: '/network', name: 'Network', component: () => import('@/components/Network.vue') },
    { path: '/settings', name: 'Settings', component: () => import('@/components/Settings.vue') },
    { path: '/info', name: 'Info', component: () => import('@/components/Info.vue') },
    { path: '/oauth_callback', component: () => import('@/components/Login.vue') },
    { path: '/:pathMatch(.*)*', redirect: '/home' },
  ],
});
