<template>
    
    <n-card title="设置">
        <n-space vertical>
            <n-alert title="技术测试版本警告" type="warning">当前版本属于非常早期的技术测试版本，可能存在很多问题，请谨慎使用。<br/>若遇到问题，请及时联系我们。</n-alert>

            <n-form>
                <n-space>
                <n-form-item label="用户密钥">
                    <n-space>
                        <n-input v-model:value="tempToken" type="password" placeholder="请输入OpenFrp用户密钥" />
                        <n-button type="primary" @click="saveSettings">
                            保存设置
                        </n-button>
                    </n-space>
                </n-form-item>
                <n-form-item label="主题">
                    <n-switch v-model:value="isDark">
                        <template #checked>
                            深色
                        </template>
                        <template #unchecked>
                            浅色
                        </template>
                    </n-switch>
                </n-form-item>
            </n-space>
            </n-form>
            <n-collapse>
                <n-collapse-item title="Frpc 管理 (首次使用请在这里下载 Frpc)" name="1">
                    <n-space>
                        <n-button @click="downloadFrpc" :loading="downloading">
                            下载 Frpc
                        </n-button>
                        <n-button @click="getFrpcVersion">
                            获取本地Frpc版本
                        </n-button>
                        <!-- <n-button @click="startFrpc" :disabled="isRunning">
                            启动
                        </n-button>
                        <n-button @click="stopFrpc" :disabled="!isRunning">
                            停止
                        </n-button> -->
                    </n-space>
                    <br />
                    <n-card title="运行日志" class="mt-4">
                        <n-log :rows="5" :log="logs" :loading="false" trim />
                    </n-card>
                </n-collapse-item>

            </n-collapse>
        </n-space>
    </n-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
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
  NCollapseItem
} from 'naive-ui'
import { inject, watch } from 'vue'

const { colorScheme, toggleColorScheme } = inject('darkMode') as any
const isDark = ref(colorScheme.value === 'dark')

watch(isDark, (newValue) => {
    if (newValue && colorScheme.value !== 'dark') {
        toggleColorScheme()
    } else if (!newValue && colorScheme.value !== 'light') {
        toggleColorScheme()
    }
})

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const message = useMessage()
const dialog = useDialog()

const downloading = ref(false)
const logs = ref('')
const userToken = ref('')
const tempToken = ref('') // 临时存储用户输入的token

// 组件加载时从localStorage读取Token
onMounted(() => {
    const savedToken = localStorage.getItem('userToken')
    if (savedToken) {
        userToken.value = savedToken
        tempToken.value = savedToken
    }
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
                downloadFrpc()
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
</script>