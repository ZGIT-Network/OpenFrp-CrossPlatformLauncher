<script setup lang="ts">
import { ref, onMounted, onUnmounted, h } from 'vue'
import { UpdateInfo } from '@/types/update'
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
    NH3,
    NStep,
    NSteps
} from 'naive-ui'
import { inject, watch, Ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { HelpCircleOutline } from '@vicons/ionicons5'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { register, unregister, isRegistered } from '@tauri-apps/plugin-deep-link'
import { openUrl } from '@tauri-apps/plugin-opener';
import Cookies from '@/utils/cookies'
import { useRouter } from 'vue-router';
import { callApi } from '@/utils/apiClient'
import dayjs from 'dayjs';
import numbro from 'numbro';
import authhelpimage from '@/assets/authhelpimage.vue'

import { logoutCurr } from '@/requests/frpApi/api2'
// import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'

// const router = useRouter()
const { colorScheme, toggleColorScheme } = inject('darkMode') as any
const isDark = ref(colorScheme.value === 'dark')
const router = useRouter();

watch(isDark, (newValue) => {
    if (newValue && colorScheme.value !== 'dark') {
        toggleColorScheme()
    } else if (!newValue && colorScheme.value !== 'light') {
        toggleColorScheme()
    }
})

const byteFormat = (num: number) => {
    return numbro(num * 1024 * 8)
        .format({ output: 'byte', base: 'general' })
        .replace('B', 'b');
};


const message = useMessage()
let downloadingMsg: ReturnType<typeof message.loading> | null = null
let downloadingPercent = ref(0)

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

    // 记录当前设置状态到日志
    logs.value += `[${new Date().toLocaleTimeString()}] [系统] 自启动状态: ${autoStart.value ? '已启用' : '未启用'}\n`
    logs.value += `[${new Date().toLocaleTimeString()}] [系统] 自动恢复隧道: ${autoRestoreTunnels.value ? '已启用' : '未启用'}\n`
    if (autoStart.value && !autoRestoreTunnels.value) {
        logs.value += `[${new Date().toLocaleTimeString()}] [系统] 警告: 自启动已启用但未开启自动恢复隧道，开机将不会自动启动隧道\n`
    }

    getCurrentVersion()
    checkAutoStartSettings()
    checkDeepLinkStatus()

    // 初始化高斯模糊特效
    if (enableGaussianBlur.value) {
        document.body.classList.add('gaussian-blur-enabled');
    } else {
        document.body.classList.remove('gaussian-blur-enabled');
    }
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
        logs.value += `[${new Date().toLocaleTimeString()}] [系统] 未检测到 frpc，需要下载\n`
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
        const checkResult = await invoke('check_and_download')
        
        // 如果需要下载，添加明确的日志
        if (checkResult === '需要下载 frpc') {
            logs.value += `[${new Date().toLocaleTimeString()}] [系统] 检测结果：未找到Frpc可执行文件\n`
        } else {
            logs.value += `[${new Date().toLocaleTimeString()}] [系统] 检测结果：Frpc可执行文件已存在\n`
            
            // 尝试获取版本信息
            try {
                const result = await invoke('get_frpc_cli_version') as string
                const frpcInfo = JSON.parse(result)
                if (frpcInfo.version && frpcInfo.version !== "未知") {
                    logs.value += `[${new Date().toLocaleTimeString()}] [系统] Frpc版本：${frpcInfo.version}\n`
                }
            } catch (e) {
                console.error('获取Frpc版本失败:', e)
            }
        }
    } catch (e) {
        logs.value += `[${new Date().toLocaleTimeString()}] [系统] 检查Frpc失败: ${e}\n`
        message.error(`检查失败: ${e}`)
    }
    
    // 从localStorage读取token，从configs.json获取自启动等设置
    // ... existing code ...
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
        const result = await invoke('get_frpc_cli_version');
        const frpcInfo = JSON.parse(result as string);
        
        if (frpcInfo.version === "未知") {
            logs.value += `[${new Date().toLocaleTimeString()}] [系统] 未找到Frpc\n`;
            message.warning('Frpc可执行文件不存在，请配置或下载');
            return;
        }
        
        logs.value += `[${new Date().toLocaleTimeString()}] [系统] 检测到Frpc版本: ${frpcInfo.version}\n`;
        message.success(`当前版本: ${frpcInfo.version}`);
    } catch (e) {
        logs.value += `[${new Date().toLocaleTimeString()}] [系统] 获取Frpc版本失败: ${e}\n`;
        message.error(`获取版本失败: ${e}`);
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
            const updateDialog = dialog.info({
                title: update.title,
                content: () =>
                    h('div', [
                        `发现新版本 v${update.latest}`,
                        h('br'), `当前版本 v${currentVersion.value}`, h('br'), h('br'),
                        '更新日志:',
                        h('br'),
                        h('pre', { style: 'white-space: pre-wrap; word-break: break-word; margin: 8px 0;' }, update.msg)
                    ]),
                positiveText: '立即更新',
                negativeText: '取消',
                onPositiveClick: async () => {
                    updateDialog.destroy();
                    try {
                        if (downloadingMsg) downloadingMsg.destroy()
                        downloadingPercent.value = 0
                        downloadingMsg = message.loading(
                          `正在下载更新...`,
                          { duration: 0 }
                        )
                        
                        await invoke('download_and_install_update')
                        if (downloadingMsg) {
                          downloadingMsg.destroy()
                          downloadingMsg = null
                        }
                        message.success('更新已下载完成，重启应用后生效')
                    } catch (e) {
                        if (downloadingMsg) {
                          downloadingMsg.destroy()
                          downloadingMsg = null
                        }
                        // message.error(`更新失败: ${e}`)
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
        
        // 如果启用自启动但未启用恢复隧道，提示用户可能需要开启
        if (autoStart.value && !autoRestoreTunnels.value) {
            setTimeout(() => {
                message.info('提示：如需开机自动启动隧道，请同时开启"开机时恢复上次运行的隧道"选项')
            }, 500)
        }
        
        if (autoStart.value && deepLinkEnabled.value) {
            setTimeout(() => {
                message.warning('注意，通过"快速启动"功能启动的隧道无法开机自启动')
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
    
    // 如果禁用了隧道恢复但启用了自启动，提示用户
    if (!value && autoStart.value) {
        setTimeout(() => {
            message.warning('已禁用开机恢复隧道，程序将在开机时启动但不会自动启动隧道')
        }, 500)
    }
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
            router.go(0);
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
            router.go(0);
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
        router.go(0);
    } catch (error: any) {
        console.error('登录失败:', error);
        message.error(error?.message || '登录失败：无效的 Authorization');
    }
}

// 添加手动放置frpc的相关功能
const appDataDir = ref('');
const manualModeVisible = ref(false);
const expectedFrpcFilename = ref('');
const frpcFullPath = ref('');

// 获取期望的frpc文件名和路径
const getExpectedFrpcInfo = async () => {
    try {
        // 尝试从后端获取现有frpc信息
        const frpcVersion = await invoke('get_frpc_cli_version') as {
            version: string,
            path: string,
            filename: string
        };

        // 如果已有文件，使用现有文件名和路径
        if (frpcVersion && frpcVersion.filename) {
            expectedFrpcFilename.value = frpcVersion.filename;
            frpcFullPath.value = frpcVersion.path;
            return;
        }
    } catch (e) {
        console.error('获取frpc信息失败:', e);
        // 忽略错误，继续执行以下代码以设置默认值
    }
        
    // 如果没有现有文件或发生错误，使用简单逻辑确定期望的文件名
    // 这与后端逻辑保持一致
    const isWindows = navigator.platform.toLowerCase().includes('win');

    if (isWindows) {
        expectedFrpcFilename.value = 'frpc_windows_amd64.exe';
    } else {
        // 检测是否为macOS
        const isMac = navigator.platform.toLowerCase().includes('mac');
        if (isMac) {
            // 检测ARM架构（M1/M2芯片）
            const isArm = /arm|aarch/i.test(navigator.platform) ||
                (/Mac/.test(navigator.userAgent) && navigator.maxTouchPoints > 1);
            expectedFrpcFilename.value = isArm ? 'frpc_darwin_arm64' : 'frpc_darwin_amd64';
        } else {
            // 检测Linux ARM架构
            const isLinuxArm = /arm|aarch/i.test(navigator.platform);
            expectedFrpcFilename.value = isLinuxArm ? 'frpc_linux_arm64' : 'frpc_linux_amd64';
        }
    }

    // 记录用于调试
    console.log("系统检测:", {
        platform: navigator.platform,
        userAgent: navigator.userAgent,
        expectedFile: expectedFrpcFilename.value
    });
};

// 获取应用数据目录
const getAppDataDir = async () => {
    try {
        appDataDir.value = await invoke('get_app_data_dir') as string;
        // 尝试获取frpc信息，但不阻止流程继续
        try {
            await getExpectedFrpcInfo();
        } catch (e) {
            console.error('获取frpc信息失败，但可以继续:', e);
            // 设置默认值以防出错
            expectedFrpcFilename.value = window.navigator.platform.includes('Win')
                ? 'frpc_windows_amd64.exe'
                : 'frpc_linux_amd64';
        }
    } catch (e) {
        console.error('获取应用数据目录失败:', e);
        message.error(`获取应用数据目录失败: ${e}`);
        // 抛出错误以便调用者处理
        throw e;
    }
};

// 打开应用数据目录
const openAppDataDir = async () => {
    try {
        await invoke('open_app_data_dir');
        message.success('已打开数据目录');
    } catch (e) {
        console.error('打开数据目录失败:', e);
        message.error(`打开数据目录失败: ${e}`);
    }
};

// 检查frpc是否存在
const checkFrpcExists = async () => {
    try {
        const result = await invoke('get_frpc_cli_version') as any;
        if (result && result.version) {
            message.success(`检测到frpc版本: ${result.version}`);
            return true;
        }
        return false;
    } catch (e) {
        console.error('检查frpc失败:', e);
        return false;
    }
};

// 显示手动放置说明
const showManualMode = async () => {
    try {
        await getAppDataDir();
        manualModeVisible.value = true;
        
        // 检查frpc是否存在
        try {
            const result = await invoke('get_frpc_cli_version') as string;
            const frpcInfo = JSON.parse(result);
            
            if (frpcInfo.version === "未知") {
                logs.value += `[${new Date().toLocaleTimeString()}] [系统] 正在配置Frpc，当前未找到可执行文件\n`;
            }
        } catch (e) {
            logs.value += `[${new Date().toLocaleTimeString()}] [系统] 未能检查Frpc状态，请按说明手动配置\n`;
        }
    } catch (e) {
        console.error('打开手动配置对话框失败:', e);
        message.error('打开手动配置对话框失败，请重试');
    }
};

onMounted(async () => {
    // ... existing code ...
    await getAppDataDir();
});

const enableBlur = ref(localStorage.getItem('enableBlur') !== 'false'); // 默认开启毛玻璃

watch(enableBlur, (val) => {
  localStorage.setItem('enableBlur', val ? 'true' : 'false');
  // 触发全局样式切换
  document.body.classList.toggle('blur-enabled', val);
});

// 高斯模糊特效开关
const enableGaussianBlur = ref(localStorage.getItem('enableGaussianBlur') !== 'false'); // 默认开启

watch(enableGaussianBlur, (val) => {
  localStorage.setItem('enableGaussianBlur', val ? 'true' : 'false');
  if (val) {
    document.body.classList.add('gaussian-blur-enabled');
  } else {
    document.body.classList.remove('gaussian-blur-enabled');
  }
});

onMounted(() => {
  // 初始化高斯模糊特效
  if (enableGaussianBlur.value) {
    document.body.classList.add('gaussian-blur-enabled');
  } else {
    document.body.classList.remove('gaussian-blur-enabled');
  }
});

// 监听自动更新进度与结果
onMounted(() => {
  listen('update-progress', (event) => {
    const percent = parseFloat(String(event.payload).replace('%', ''))
    if (!isNaN(percent)) {
      downloadingPercent.value = Math.round(percent)
      if (downloadingMsg) {
        downloadingMsg.content = () => `正在下载更新... ${downloadingPercent.value}%`
      }
    }
  })
  listen('update-error', (event) => {
    if (downloadingMsg) {
      downloadingMsg.destroy()
      downloadingMsg = null
    }
    // message.error(event.payload as string)
  })
  listen('update-success', (event) => {
    if (downloadingMsg) {
      downloadingMsg.destroy()
      downloadingMsg = null
    }
    // message.success(event.payload as string || '更新已下载完成，重启应用后生效')
  })
});
</script>

<template>
    <n-scrollbar>
        <n-space vertical>

            <n-alert type="warning">您当前正在使用 Beta 测试版本，可能存在一些问题，请谨慎在生产环境使用。<br />若遇到问题，请及时反馈。</n-alert>

            <n-card title="已通过 NatayarkID 登录" v-if="userToken" hoverable style="height: 100%">
                <template #header-extra>
                    <n-button type="tertiary" @click="logout">退出登录</n-button>
                </template>
                <n-thing style="width: 100%">
                    <template #avatar>
                        <n-tooltip :show-arrow="false" trigger="hover" placement="bottom">
                            <template #trigger>
                                <n-avatar :size="48"
                                    :src="'https://api.zyghit.cn/avatar/?email=' + userInfo?.email + '&s=256'"
                                    fallback-src="https://07akioni.oss-cn-beijing.aliyuncs.com/07akioni.jpeg" />
                            </template>
                            更换头像？请前往 Gravatar !
                        </n-tooltip>
                    </template>
                    <template #header>
                        #{{ userInfo?.id }} {{ userInfo?.username }} <n-tag type="info">{{ userInfo?.friendlyGroup
                            }}</n-tag>
                    </template>
                    <template #description>
                        {{ userInfo?.email }}
                    </template>

                    <n-flex style="height: 100%" justify="space-between" vertical>

                        <n-descriptions :column="2">



                            <n-descriptions-item label="隧道数">{{ userInfo?.used }} / {{ userInfo?.proxies }}
                            </n-descriptions-item>

                            <n-descriptions-item label="带宽速率 (上 / 下)">
                                <span>{{ byteFormat(userInfo?.outLimit || 0).replace('Mb', '') }} Mbps</span>
                                /
                                <span>{{ byteFormat(userInfo?.inLimit || 0).replace('Mb', '') }} Mbps</span>
                            </n-descriptions-item>

                            <n-descriptions-item label="注册时间">
                                {{ userInfo?.regTime }}

                            </n-descriptions-item>
                            <n-descriptions-item label="剩余流量">{{
                                numbro((userInfo?.traffic || 0) * 1024 * 1024).format({
                                    output: 'byte',
                                    base: 'binary',
                                    mantissa: 2,
                                })
                            }}
                            </n-descriptions-item>
                        </n-descriptions>
                        <n-space vertical :size="[0, 8]">
                            <n-text :depth="3">签到可以获得免费流量，打开网页面板立即完成每日签到。
                            </n-text>
                        </n-space>
                    </n-flex>
                </n-thing>




            </n-card>

            <n-card title="设置">
                <n-space vertical>


                    <n-form>
                        <n-space>

                            <n-form-item v-if="!userToken" label="用户登录">
                                <n-space>
                                    <!-- <n-input v-model:value="tempToken" type="password" placeholder="请输入OpenFrp访问密钥" />
                                <n-button type="primary" @click="saveSettings">保存设置</n-button> -->

                                    <n-tabs type="bar" animated>
                                        <n-tab-pane name="oauth" tab="通过 NatayarkID 登录 ">
                                            <n-button type="primary" @click="oauthLogin">OAuth 登录</n-button>


                                        </n-tab-pane>
                                        <n-tab-pane name="authorization" tab="通过 Authorization 登录">
                                            <n-form-item-row label="请输入在面板获取的 Authorization">
                                                <n-input v-model:value="Authorization" type="password"
                                                    placeholder="Authorization" />
                                                <n-button quaternary circle @click="helpDrawer('authorization')">
                                                    <template #icon>
                                                        <n-icon>
                                                            <HelpCircleOutline />
                                                        </n-icon>
                                                    </template>
                                                </n-button>
                                            </n-form-item-row>
                                            <n-button type="primary" @click="AuthLogin">登录</n-button>


                                        </n-tab-pane>
                                    </n-tabs>

                                </n-space>
                            </n-form-item>

                                <n-text>主题切换已移至窗口右上角</n-text>
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
                                    <n-button @click="openAppDataDir">
                                        打开软件数据目录
                                    </n-button>
                                </n-space>
                                <!-- 添加新按钮：打开数据目录 -->

                            </n-space>
                        </n-collapse-item>
                        <n-collapse-item title="Frpc 管理" name="1">
                            <template #header-extra>
                                首次使用请在这里下载或配置 Frpc
                            </template>
                            <n-space>
                                <n-button @click="downloadFrpc" :loading="downloading" :disabled="downloading">
                                    {{ downloading ? '正在进行操作...' : '自动下载/更新 Frpc' }}
                                </n-button>
                                <n-button @click="getFrpcVersion" :disabled="downloading">获取本地 Frpc 版本</n-button>

                                <!-- 添加新按钮：手动放置按钮 -->
                                <n-button @click="showManualMode" :disabled="downloading">
                                    手动配置 Frpc 可执行文件
                                </n-button>



                                <n-popconfirm @positive-click="killAllProcesses" :disabled="downloading">
                                    <template #trigger>
                                        <n-button type="warning" :disabled="downloading">终止所有 Frpc 进程</n-button>
                                    </template>
                                    确认终止所有 Frpc 进程？这将会断开所有连接
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
                                    <n-switch v-model:value="autoRestoreTunnels"
                                        @update:value="toggleAutoRestoreTunnels" />
                                    <span>开机时恢复上次运行的隧道</span>
                                </n-space>
                                <n-space align="center">
                                    <n-tooltip trigger="hover">
                                        <template #trigger>
                                            <n-switch v-model:value="deepLinkEnabled" :disabled="true"
                                                @update:value="toggleDeepLink" />

                                        </template>
                                        允许通过"快速启动"链接启动隧道
                                    </n-tooltip>
                                    <span>启用"快速启动"功能 </span><n-button quaternary circle
                                        @click="helpDrawer('quickstart')">
                                        <template #icon>
                                            <n-icon>
                                                <HelpCircleOutline />
                                            </n-icon>
                                        </template>
                                    </n-button>
                                </n-space>
                                <!-- 高斯模糊特效开关 -->
                                <n-space align="center">
                                    <n-switch v-model:value="enableGaussianBlur" disabled/>
                                    <span>高斯模糊视觉特效（窗口需支持透明）</span>
                                </n-space>
                            </n-space>
                        </n-collapse-item>
                    </n-collapse>
                </n-space>
            </n-card>
        </n-space>

        <n-drawer v-model:show="helpDrawerVisible" width="40%" placement="right">
            <n-drawer-content closable>
                <template #header>
                    功能帮助指南
                </template>
                <n-thing v-if="helpDrawerContent === 'quickstart'">
                    <n-h3>快速启动</n-h3>
                    <n-text>
                        快速启动 是一种基于注册链接(deep link)快速启动隧道的方式
                        <br />通过在面板简单的点击链接，即可快速启动隧道
                        <br />
                        <br />
                        * 通过"快速启动"功能启动的隧道无法开机自启动
                    </n-text>
                </n-thing>

                <n-thing v-if="helpDrawerContent === 'authorization'">
                    <n-h3>通过 Authorization 登录</n-h3>
                    <n-text>
                        可在 网页面板-个人中心 中的 "第三方客户端安全登录" 功能获取 Authorization 会话密钥。
                        <br />在无法使用Oauth回调登录时可尝试使用本方案。
                        <br />
                        <authhelpimage />

                        <br />
                        * 虽然说是官方客户端，但是还是这样最简单来说
                    </n-text>
                </n-thing>
                <n-thing v-if="helpDrawerContent === 'none'">
                    你打开了一个什么都没有的提示框？
                </n-thing>

            </n-drawer-content>
        </n-drawer>

        <!-- 使用NDrawer代替NModal -->
        <n-drawer v-model:show="manualModeVisible" :width="600" placement="right">
            <n-drawer-content title="手动配置 Frpc 可执行文件" closable>
                <n-space vertical>
                    <n-alert type="info">
                        如果自动下载失败，您可以手动下载 Frpc 可执行文件并放置到程序数据目录
                    </n-alert>

                    <n-alert type="warning" title="注意">
                        请在 OpenFrp 管理面板 - 下载中心 下载<b>对应操作系统和对应平台的</b> Frpc 可执行文件。 <br />
                        <br />
                        下载后的文件一般为压缩格式，请解压后复制到数据目录，目录结构如下：
                        <br /> <br />
                        <n-code>
                        com.of-cpl.app/  <br />
                            - /frpc <br />
                            - /config <br />
                            - config.json <br />
                            + frpc_windows_amd64.exe (解压后复制到数据目录的文件) <br />
                        </n-code>
                        <br />
                        * 请确保文件名与输入的文件名一致。在部分操作系统下，可能需要手动配置可执行权限。
                    </n-alert>

                    <n-form-item label="应用数据目录">
                        <n-input v-model:value="appDataDir" readonly />
                        <n-button @click="openAppDataDir">
                            打开数据目录
                        </n-button>
                    </n-form-item>
                    

                    <n-form-item label="Frpc 可执行文件名称">
                        <n-input v-model:value="expectedFrpcFilename" readonly />

                    </n-form-item>

                    


                </n-space>
                <template #footer><n-space justify="end">
                        <n-button @click="manualModeVisible = false">
                            关闭
                        </n-button>
                        <n-button type="primary" @click="getFrpcVersion(); manualModeVisible = false">
                            完成并检查
                        </n-button>
                    </n-space></template>
            </n-drawer-content>
        </n-drawer>
    </n-scrollbar>
</template>
