import { createRouter, createWebHashHistory } from 'vue-router'
import FrpcManager from '../components/FrpcManager.vue'
import Settings from '../components/Settings.vue'
import Home from '../components/Home.vue'
import ProxyList from '../components/ProxyList.vue'
import Info from '../components/Info.vue'
import CreateProxy from '../components/CreateProxies/index.vue'
import Login from '@/components/Login.vue'
// import NodeStatus from '@/components/NodeStatus.vue.bak'

const routes = [
  {
    path: '/',
    redirect: '/home'
  },
  {
    path: '/home',
    name: 'Home',
    component: Home
  },
  {
    path: '/createproxy',
    name: 'CreateProxy',
    component: CreateProxy
  },
  {
    path: '/proxylist',
    name: 'ProxyList',
    component: ProxyList
  },
  {
    path: '/login',
    name: 'Login',
    component: Login
  },
  {
    path: '/frpc',
    name: 'FrpcManager',
    component: FrpcManager
  },
  // {
  //   path: '/status',
  //   name: 'NodeStatus',
  //   component: NodeStatus
  // },
  {
    path: '/settings',
    name: 'Settings',
    component: Settings
  },
  {
    path: '/info',
    name: 'Info',
    component: Info
  },
  {
    path: '/oauth_callback',
    component: Login,
  },
]

const router = createRouter({
  // 使用 hash 模式而不是 history 模式，因为这更适合桌面应用
  history: createWebHashHistory(),
  routes
})

export default router 