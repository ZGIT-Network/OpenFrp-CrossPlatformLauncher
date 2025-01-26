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
    NScrollbar
} from 'naive-ui'
import { inject, watch } from 'vue'
import { useRouter, onBeforeRouteLeave } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'

const router = useRouter()
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
const userToken = ref('')
const tempToken = ref('') // 临时存储用户输入的token
const currentVersion = ref('获取中...')
const checking = ref(false)
const autoStart = ref(false)

const activeNames = ref<string[]>(['2']); // 控制展开的项

// 添加路由守卫
onBeforeRouteLeave((to, from, next) => {
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
    getCurrentVersion()
    checkAutoStart()
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
watch(userToken, (newValue) => {
    localStorage.setItem('userToken', newValue)
})

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
        const version = await invoke('get_cpl_version')
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
        const update = await invoke('check_update')
        // console.log('更新信息:', update)

        if (update) {  // update 是 CplUpdate 对象
            dialog.info({
                title: update.title,
                content: () =>
                    h('div', [
                        `发现新版本 ${update.latest}`,
                        h('br'), h('br'),
                        '更新日志:',
                        h('br'),
                        ...update.msg.split('\n').map((line, index) => h('div', { key: index }, line))
                    ]),
                positiveText: '立即更新',
                negativeText: '取消',
                onPositiveClick: async () => {
                    try {
                        await invoke('download_and_update')
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

// 检查自启动状态
const checkAutoStart = async () => {
    try {
        autoStart.value = await invoke('check_auto_start')
    } catch (e) {
        console.error('检查自启动状态失败:', e)
    }
}

// 切换自启动
const toggleAutoStart = async () => {
    try {
        await invoke('toggle_auto_start', { enable: autoStart.value })
        // 切换后重新检查状态
        await checkAutoStart()
        message.success(`${autoStart.value ? '启用' : '禁用'}开机自启动成功`)
    } catch (e) {
        message.error(`设置开机自启动失败: ${e}`)
        // 发生错误时也重新检查状态
        await checkAutoStart()
    }
}
</script>

<template>
    <n-scrollbar>
        <n-card title="设置">
            <n-space vertical>
                <n-alert title="技术测试版本警告" type="warning">
                    当前版本属于非常早期的技术测试版本，可能存在很多问题，请谨慎使用。<br />
                    若遇到问题，请及时联系我们。
                </n-alert>

                <n-form>
                    <n-space>
                        <n-form-item label="用户密钥">
                            <n-space>
                                <n-input v-model:value="tempToken" type="password" placeholder="请输入OpenFrp访问密钥" />
                                <n-button type="primary" @click="saveSettings">保存设置</n-button>
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

                <n-collapse v-model:expanded-names="activeNames">
                    <n-collapse-item title="版本信息" name="2">
                        <n-space vertical>
                            <n-text>当前版本：v{{ currentVersion }}</n-text>
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
                        </n-space>
                    </n-collapse-item>
                </n-collapse>
            </n-space>
        </n-card>
    </n-scrollbar>
</template>
