<script setup lang="ts">
import { ref, onMounted, onUnmounted, h } from 'vue'
import {
    useMessage,
    useDialog,
    NButton,
    NCard,
    NLog,
    NSpace,
    NAlert,
    NForm,
    NFormItem,
    NSwitch,
    NInput,
    NCollapse,
    NCollapseItem,
    NPopconfirm,
    NText,
    NScrollbar,
    NTooltip,
    NDrawer,
    NDrawerContent,
    NThing,
    NIcon,
    NH3
    
} from 'naive-ui'
import { inject, watch, Ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { HelpCircleOutline } from '@vicons/ionicons5'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { register, unregister, isRegistered } from '@tauri-apps/plugin-deep-link'
import { openUrl } from '@tauri-apps/plugin-opener';
import Cookies from '@/utils/cookies'
import { callApi } from '@/utils/apiClient'
import authhelpimage from '@/assets/authhelpimage.vue'

import { logoutCurr } from '@/requests/frpApi/api2'
// import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'

// const router = useRouter()
const { colorScheme, toggleColorScheme } = inject('darkMode') as any
const isDark = ref(colorScheme.value === 'dark')

watch(isDark, (newValue) => {
    if (newValue && colorScheme.value !== 'dark') {
        toggleColorScheme()
    } else if (!newValue && colorScheme.value !== 'light') {
        toggleColorScheme()
    }
})

const message = useMessage()
const dialog = useDialog()

const downloading = ref(false)
const logs = ref('')

const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');
const userInfo = userInfoObj?.userInfo;
const Authorization = ref('')



console.log(userInfo)

const userToken = ref(userInfo?.value?.token)
const tempToken = ref('') // 临时存储用户输入的token
const currentVersion = ref('获取中...')
const checking = ref(false)
const autoStart = ref(false)
const autoRestoreTunnels = ref(true)  // 默认设为 true
const deepLinkEnabled = ref(false)
const helpDrawerVisible = ref(false)

const activeNames = ref<string[]>(['2']); // 控制展开的项

// 添加路由守卫
onBeforeRouteLeave((_to, _from, next) => {
    if (downloading.value) {
        dialog.warning({
            title: '提示',
            content: '正在下载 frpc，离开页面将中断下载。确定要离开吗？',
            positiveText: '继续下载',
            negativeText: '离开',
            onPositiveClick: () => { next(false) },
            onNegativeClick: () => { next() }
        })
    } else {
        next()
    }
})

// 组件加载时从localStorage读取Token
onMounted(() => {
    const savedToken = localStorage.getItem('userToken')
    if (savedToken) {
        userToken.value = savedToken
        tempToken.value = savedToken
    }

    Authorization.value = Cookies.get('authorization') || ''

    // 设置默认值
    if (localStorage.getItem('autoRestoreTunnels') === null) {
        localStorage.setItem('autoRestoreTunnels', 'true')
    }
    autoRestoreTunnels.value = localStorage.getItem('autoRestoreTunnels') === 'true'

    // 添加自动启动状态的持久化
    const savedAutoStart = localStorage.getItem('autoStart')
    if (savedAutoStart !== null) {
        autoStart.value = savedAutoStart === 'true'
    }

    getCurrentVersion()
    checkAutoStartSettings()
    checkDeepLinkStatus()
})

// 监听自动启动状态变化
watch(autoStart, (newValue) => {
    localStorage.setItem('autoStart', newValue.toString())
})
// 保存设置
const saveSettings = () => {
    userToken.value = tempToken.value
    localStorage.setItem('userToken', userToken.value)
    message.success('设置已保存')
}

// 监听日志事件
let unlistenLog: any = null
let unlistenNeedDownload: any = null

// 将用户Token保存到localStorage

onMounted(async () => {
    unlistenLog = await listen('log', (event: any) => {
        logs.value += event.payload.message + '\n'
    })

    unlistenNeedDownload = await listen('need_download', async () => {
        dialog.warning({
            title: '提示',
            content: '未检测到 frpc，是否立即下载？',
            positiveText: '下载',
            negativeText: '取消',
            onPositiveClick: () => {
                downloadFrpc();
            }
        })
    })

    // 启动时检查 frpc
    try {
        await invoke('check_and_download')
    } catch (e) {
        message.error(`检查失败: ${e}`)
    }
})

onUnmounted(() => {
    if (unlistenLog) {
        unlistenLog()
    }
    if (unlistenNeedDownload) {
        unlistenNeedDownload()
    }
})

const downloadFrpc = async () => {
    activeNames.value = ['1']  // 设置展开的项为 Frpc 管理
    downloading.value = true
    try {
        await invoke('download_frpc')
        message.success('下载成功')
    } catch (e) {
        message.error(`下载失败: ${e}`)
    } finally {
        downloading.value = false
    }
}

const getFrpcVersion = async () => {
    try {
        await invoke('get_frpc_cli_version')
        message.success('版本获取成功')
    } catch (e) {
        message.error(`获取版本失败: ${e}`)
    }
}

const killAllProcesses = async () => {
    try {
        await invoke('kill_all_processes')
        message.success('已终止所有 frpc 进程')
    } catch (e) {
        message.error(`操作失败: ${e}`)
    }
}

// 获取当前版本
const getCurrentVersion = async () => {
    try {
        const version = await invoke('get_cpl_version') as string
        currentVersion.value = version
    } catch (e) {
        currentVersion.value = '获取失败'
        console.error('获取版本失败:', e)
    }
}

// 检查更新
const checkUpdate = async () => {
    checking.value = true
    try {
        const update = await invoke('check_update') as CplUpdate
        if (update) {
            dialog.info({
                title: update.title,
                content: () =>
                    h('div', [
                        `发现新版本 v${update.latest}`,
                        h('br'), `当前版本 v${currentVersion.value}`, h('br'), h('br'),
                        '更新日志:',
                        h('br'),
                        ...update.msg.split('\n').map((line, index) => h('div', { key: index }, line))
                    ]),
                positiveText: '立即更新',
                negativeText: '取消',
                onPositiveClick: async () => {
                    try {
                        message.loading('正在下载更新...', { duration: 0 })
                        await invoke('download_and_update')
                        dialog.success({
                            title: '更新下载完成',
                            content: '更新已下载完成，重启应用后生效',
                            positiveText: '立即重启',
                            onPositiveClick: async () => {
                                await invoke('exit_app')
                            }
                        })
                    } catch (e) {
                        message.error(`更新失败: ${e}`)
                    }
                }
            })
        } else {
            message.success('已是最新版本')
        }
    } catch (e) {
        console.error('检查更新错误:', e)
        message.error('检查更新失败，请稍后重试')
    } finally {
        checking.value = false
    }
}

// 检查自启动状态和恢复隧道设置
const checkAutoStartSettings = async () => {
    try {
        autoStart.value = await invoke('check_auto_start')
        // 从 localStorage 读取恢复隧道设置
        const savedValue = localStorage.getItem('autoRestoreTunnels')
        autoRestoreTunnels.value = savedValue === null ? true : savedValue === 'true'
    } catch (e) {
        console.error('检查自启动状态失败:', e)
    }
}

// 切换自启动
const toggleAutoStart = async () => {
    try {
        await invoke('toggle_auto_start', { enable: autoStart.value })
        // 切换后重新检查状态
        await checkAutoStartSettings()
        message.success(`${autoStart.value ? '启用' : '禁用'}开机自启动成功`)
        if (autoStart.value && deepLinkEnabled.value) {
            setTimeout(() => {
                message.warning('注意，通过“快速启动”功能启动的隧道无法开机自启动')
            }, 200)
        }
    } catch (e) {
        message.error(`设置开机自启动失败: ${e}`)
        // 发生错误时也重新检查状态
        await checkAutoStartSettings()
    }
}

// 切换恢复隧道设置
const toggleAutoRestoreTunnels = (value: boolean) => {
    autoRestoreTunnels.value = value
    localStorage.setItem('autoRestoreTunnels', value.toString())
    message.success(`${value ? '启用' : '禁用'}开机恢复隧道成功`)
}

// 检查深层链接状态
const checkDeepLinkStatus = async () => {
    try {
        if (navigator.platform.includes('Win') || navigator.platform.includes('Linux')) {
            deepLinkEnabled.value = await isRegistered('openfrp')
        }
    } catch (e) {
        // 如果还没有注册过，isRegistered 会抛出错误，这是正常的
        deepLinkEnabled.value = false
        console.debug('检查深层链接状态:', e)
    }
}

// 切换深层链接
const toggleDeepLink = async (value: boolean) => {
    try {
        if (navigator.platform.includes('Win') || navigator.platform.includes('Linux')) {
            if (value) {
                await register('openfrp')
                message.success('启用快速启动功能成功')
            } else {
                await unregister('openfrp')
                message.success('禁用快速启动功能成功')
            }
            await checkDeepLinkStatus()
        }
    } catch (e) {
        message.error(`设置快速启动功能失败: ${e}`)
        await checkDeepLinkStatus()
    }
}

// const router = useRouter()
import getLoginUrl from '@/requests/oauth/getLoginUrl'

const oauthLogin = () => {
    message.loading('正在准备登录...', { duration: 3000 });
    getLoginUrl()
        .then((res) => {
          if (!res.data.flag) {
            message.error('无法获取登录URL: ' + (res.data.msg || '未知错误'));
            return;
          }
          
          try {
            // 在打开URL前显示提示
            message.info('正在打开登录页面，请在浏览器中完成授权');
            // 使用 setTimeout 确保消息显示后再打开URL
            setTimeout(() => {
              openUrl(res.data.data)
                .catch(err => {
                  console.error('打开URL失败:', err);
                  message.error('无法打开浏览器，请手动复制链接进行登录');
                  // 提供复制链接的选项
                  dialog.info({
                    title: '手动登录',
                    content: '请复制以下链接在浏览器中打开完成登录:',
                    action: () => h(NInput, {
                      value: res.data.data,
                      readonly: true,
                      onFocus: (e: FocusEvent) => {
                        const target = e.target as HTMLInputElement;
                        target?.select();
                      }
                    })
                  });
                });
            }, 500);
          } catch (error) {
            console.error('打开URL过程中出错:', error);
            message.error('打开登录页面失败，请稍后重试');
          }
        })
        .catch((err) => {
          console.error('获取登录URL失败:', err);
          message.error('请求登录URL时发生错误: ' + (err.message || '未知错误'));
        });
}

interface CplUpdate {
    title: string;
    latest: string;
    msg: string;
}

const helpDrawerContent = ref('none')
const helpDrawer = (type: string) => {
    helpDrawerVisible.value = true
    helpDrawerContent.value = type
}
// 添加退出登录函数
const logout = () => {
  
  dialog.warning({
    title: '确认退出',
    content: '确定要退出登录吗？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: () => {
      // 清除用户token
      logoutCurr()
      userToken.value = ''
      tempToken.value = ''
      Cookies.remove('authorization');
      localStorage.removeItem('userToken')
      message.success('已成功退出登录')
      window.location.reload()
    }
  })
}

const Authlogout = () => {
  
  dialog.warning({
    title: '确认退出',
    content: '确定要退出登录吗？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: () => {
      // 清除用户token
      logoutCurr()
      Cookies.remove('authorization');
      userToken.value = ''
      tempToken.value = ''
      localStorage.removeItem('userToken')
      message.success('已成功退出登录')
      window.location.reload()
    }
  })
}
const AuthLogin = async () => {
  if (!Authorization.value) {
    message.error('请输入 Authorization');
    return;
  }
  message.loading('正在登录...', { duration: 2000 });
  
  try {
    // 直接使用 invoke 而不是 callApi，避免循环检查
    const testResponse = await invoke('proxy_api', {
      url: 'getUserInfo',
      method: 'POST',
      headers: {
        Authorization: Authorization.value
      },
      body: {},
    });
    console.log(testResponse);

    if (!testResponse || !(testResponse as any).flag) {
      message.error((testResponse as any)?.msg || '登录失败：无效的 Authorization');
      return;
    }

    // Authorization 有效，保存登录状态
    Cookies.set('authorization', Authorization.value, {
      expires: 7,
    });
    userToken.value = Authorization.value;
    localStorage.setItem('userToken', Authorization.value);
    tempToken.value = Authorization.value;
    message.success('登录成功');
    window.location.reload();
  } catch (error: any) {
    console.error('登录失败:', error);
    message.error(error?.message || '登录失败：无效的 Authorization');
  }
}
</script>

<template>
    <n-scrollbar>
        <n-card title="设置">
            <n-space vertical>
                <n-alert type="warning">您当前正在使用 Beta 测试版本，可能存在一些问题，请谨慎在生产环境使用。<br />若遇到问题，请及时反馈。</n-alert>


                <n-form>
                    <n-space>
                        <n-form-item label="用户登录">
                            <n-space>
                                <!-- <n-input v-model:value="tempToken" type="password" placeholder="请输入OpenFrp访问密钥" />
                                <n-button type="primary" @click="saveSettings">保存设置</n-button> -->
                                <n-space v-if="userToken">
                                    已登录至: {{userInfo?.username}}
                                    <n-button type="error" @click="logout">退出登录</n-button>
                                  </n-space>
                                <n-tabs v-else type="bar" animated >
                                    <n-tab-pane name="oauth" tab="通过 NatayarkID 登录 ">
                                        <n-button v-if="!userToken" type="primary" @click="oauthLogin">oauth登录</n-button>

                                        
                                    </n-tab-pane>
                                    <n-tab-pane name="authorization" tab="通过 Authorization 登录">
                                        <n-form-item-row label="请输入在面板获取的 Authorization"> 
                                            <n-input v-model:value="Authorization" type="password" placeholder="Authorization" />
                                            <n-button quaternary circle  @click="helpDrawer('authorization')">
                                                <template #icon>
                                                    <n-icon><HelpCircleOutline /></n-icon>
                                                </template>
                                            </n-button>
                                        </n-form-item-row>
                                         <n-button v-if="!userToken"  type="primary" @click="AuthLogin">登录</n-button>

                                    
                                    </n-tab-pane>
                                </n-tabs>
                                
                            </n-space>
                        </n-form-item>
                        
                        <n-form-item label="主题">
                            <n-switch v-model:value="isDark">
                                <template #checked>深色</template>
                                <template #unchecked>浅色</template>
                            </n-switch>
                        </n-form-item>
                    </n-space>
                </n-form>

                <n-collapse v-model:expanded-names="activeNames" accordion>
                    <n-collapse-item title="版本信息" name="2">
                        <n-space vertical>
                            <n-text>当前版本：Beta v{{ currentVersion }}</n-text>
                            <n-space>
                                <n-button @click="checkUpdate" :loading="checking">
                                    {{ checking ? '检查中...' : '检查更新' }}
                                </n-button>
                            </n-space>
                        </n-space>
                    </n-collapse-item>
                    <n-collapse-item title="Frpc 管理" name="1">
                        <template #header-extra>
                            首次使用请在这里下载 Frpc
                        </template>
                        <n-space>
                            <n-button @click="downloadFrpc" :loading="downloading" :disabled="downloading">
                                {{ downloading ? '正在进行操作...' : '检查更新或者下载 Frpc' }}
                            </n-button>
                            <n-button @click="getFrpcVersion" :disabled="downloading">获取本地Frpc版本</n-button>
                            <n-popconfirm @positive-click="killAllProcesses" :disabled="downloading">
                                <template #trigger>
                                    <n-button type="warning" :disabled="downloading">终止所有 frpc 进程</n-button>
                                </template>
                                确认终止所有 frpc 进程？这将会断开所有连接
                            </n-popconfirm>
                        </n-space>
                        <br />
                        <n-card title="运行日志" class="mt-4">
                            <n-log :rows="10" :log="logs" :loading="false" trim />
                        </n-card>
                    </n-collapse-item>
                    <n-collapse-item title="启动设置" name="3">
                        <n-space vertical>
                            <n-space align="center">
                                <n-switch v-model:value="autoStart" @update:value="toggleAutoStart" />
                                <span>开机自启动</span>
                            </n-space>
                            <n-space align="center" v-if="autoStart">
                                <n-switch v-model:value="autoRestoreTunnels" @update:value="toggleAutoRestoreTunnels" />
                                <span>开机时恢复上次运行的隧道</span>
                            </n-space>
                            <n-space align="center">
                                <n-tooltip trigger="hover">
                                    <template #trigger>
                                        <n-switch v-model:value="deepLinkEnabled"  :disabled="true"  @update:value="toggleDeepLink" />
                                       
                                    </template>
                                    允许通过“快速启动”链接启动隧道
                                </n-tooltip>
                                <span>启用"快速启动"功能 </span><n-button quaternary circle  @click="helpDrawer('quickstart')">
                                    <template #icon>
                                        <n-icon><HelpCircleOutline /></n-icon>
                                    </template>
                                </n-button>
                            </n-space>
                        </n-space>
                    </n-collapse-item>
                </n-collapse>
            </n-space>
        </n-card>
        <n-drawer v-model:show="helpDrawerVisible" width="40%" placement="right">
            <n-drawer-content closable>
                <template #header>
                    功能帮助指南
                </template>
                <n-thing v-if="helpDrawerContent === 'quickstart'">
                    <n-h3>快速启动</n-h3>
                    <n-text>
                       快速启动 是一种基于注册链接(deep link)快速启动隧道的方式
                    <br/>通过在面板简单的点击链接，即可快速启动隧道
                    <br/>
                    <br/>
                        * 通过“快速启动”功能启动的隧道无法开机自启动
                    </n-text>
                </n-thing>

                <n-thing v-if="helpDrawerContent === 'authorization'">
                    <n-h3>通过 Authorization 登录</n-h3>
                    <n-text>
                       可在 网页面板-个人中心 中的 “第三方客户端安全登录” 功能获取 Authorization 会话密钥。
                    <br/>在无法使用Oauth回调登录时可尝试使用本方案。
                    <br/>
                    <authhelpimage/>

                    <br/>
                        * 虽然说是官方客户端，但是还是这样最简单来说
                    </n-text>
                </n-thing>
                <n-thing v-if="helpDrawerContent === 'none'">
                    你打开了一个什么都没有的提示框？
                </n-thing>
              
            </n-drawer-content>
          </n-drawer>
    </n-scrollbar>
</template>
