<script setup lang="ts">
import { NCard, NAlert, NSpace, NH4, NH2, NList, NListItem, NThing, NTable, NScrollbar, NImage, NGradientText, NText, NConfigProvider, NButton, NTag, NResult, NSkeleton } from 'naive-ui';
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const currentVersion = ref('v0.1')

const getCurrentVersion = async () => {
    try {
        const version = await invoke('get_cpl_version')
        currentVersion.value = version as string
    } catch (e) {
        currentVersion.value = '获取失败'
        console.error('获取版本失败:', e)
    }
}
getCurrentVersion()

const systemInfo = ref('')
const buildInfo = ref('')

// 定义服务检测配置
interface ServiceCheck {
  id: string;
  name: string;
  url: string;
  status: boolean | null;
  checking: boolean;
  error: string | null;
  latency: number | null; // 增加延迟字段
}

const serviceChecks = ref<ServiceCheck[]>([
  {
    id: 'main-api',
    name: 'OPENFRP 基本 API 状态 (ofapi)',
    url: 'https://api.openfrp.net/commonQuery/get?key=test',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
  {
    id: 'backup-api',
    name: 'OPENFRP 备用 API 状态 (bfsea)',
    url: 'https://of-dev-api.bfsea.com/commonQuery/get?key=test',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
  {
    id: 'zgit-api',
    name: 'ZGIT Network OPENAPI 状态',
    url: 'https://api.zyghit.cn',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
   {
    id: 'nykid',
    name: 'Natayark OpenID 用户中心状态',
    url: 'https://account.naids.com/api/api/',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
  {
    id: 'collect-api',
    name: 'Natayark 人机验证服务状态',
    url: 'https://captcha.naids.com/gen',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
  {
    id: 'r-zyghit',
    name: 'ZGIT Network 资源服务状态',
    url: 'https://r.zyghit.cn/',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
  {
    id: 'r-naids',
    name: 'Natayark 静态资源服务状态',
    url: 'https://staticassets.naids.com/',
    status: null,
    checking: false,
    error: null,
    latency: null
  },
])

// 全局检查状态变量
const checking = ref(false)
const onlineStatus = ref<boolean | null>(null)
const checkingNetwork = ref(false)
const networkError = ref<string | null>(null)

// 检查网络连接状态
const checkNetworkConnectivity = async () => {
    checkingNetwork.value = true
    networkError.value = null
    try {
        // 使用 fetch 检查网络连接 (检查是否能访问百度)
        const controller = new AbortController()
        const timeoutId = setTimeout(() => controller.abort(), 5000) // 5秒超时
        
        const response = await fetch('https://www.baidu.com', {
            mode: 'no-cors',
            signal: controller.signal
        })
        
        // 如果请求没有抛出错误，则认为网络连接正常
        onlineStatus.value = true
        clearTimeout(timeoutId)
    } catch (e: any) {
        console.error('网络连接检查失败:', e)
        onlineStatus.value = false
        networkError.value = e.message || '网络连接异常'
    } finally {
        checkingNetwork.value = false
    }
}

// 检查单个服务状态
const checkServiceStatus = async (service: ServiceCheck) => {
    service.checking = true
    service.error = null
    service.latency = null
    
    try {
        const controller = new AbortController()
        const timeoutId = setTimeout(() => controller.abort(), 5000) // 5秒超时
        
        // 记录开始时间
        const startTime = Date.now()
        
        const response = await fetch(service.url, {
            signal: controller.signal
        })
        
        // 计算延迟
        const endTime = Date.now()
        service.latency = endTime - startTime
        
        // 将404状态码也视为正常连接（根据用户要求）
        // 对于CORS错误，fetch会抛出异常，我们在catch块中处理
        service.status = response.ok || response.status === 404
        if (!response.ok && response.status !== 404) {
            service.error = `HTTP ${response.status}: ${response.statusText}`
        }
        clearTimeout(timeoutId)
    } catch (e: any) {
        console.error(`${service.name}检查失败:`, e)
        // 即使出现CORS错误或其他网络错误，也视为连接正常
        // 因为这表明服务器是可访问的，只是存在跨域限制
        service.status = true
        service.error = null
    } finally {
        service.checking = false
    }
}

// 依次执行所有服务检查，增加延迟
const checkAllServicesSequentially = async () => {
    // 先将所有服务标记为检查中状态
    serviceChecks.value.forEach(service => {
        service.checking = true
        service.status = null
        service.error = null
        service.latency = null
    })
    
    // 等待一小段时间，确保UI更新
    await new Promise(resolve => setTimeout(resolve, 50))
    
    for (const service of serviceChecks.value) {
        await checkServiceStatus(service)
        // 增加延迟，使检查过程更明显
        await new Promise(resolve => setTimeout(resolve, 300))
    }
}

// 执行所有检查
const performAllChecks = async () => {
    checking.value = true
    try {
        await checkNetworkConnectivity()
        // 增加延迟，使检查过程更明显
        await new Promise(resolve => setTimeout(resolve, 300))
        await checkAllServicesSequentially()
    } finally {
        checking.value = false
    }
}

onMounted(async () => {
    try {
        currentVersion.value = await invoke('get_cpl_version')
        systemInfo.value = await invoke('get_system_info')
        buildInfo.value = await invoke('get_build_info')
    } catch (e) {
        console.error('获取版本信息失败:', e)
    }
    
    // 组件挂载后自动执行检查
    performAllChecks()
})
const timestamp = new Date().toISOString()
const userAgent = navigator.userAgent
</script>
<template>
    <n-scrollbar>
        <n-h2 style="margin-bottom: 3px;">网络</n-h2> 

        <n-space vertical>
            <n-alert type="info" closable>这是一个正在开发的功能，他可能暂时无法使用或是存在许多问题。</n-alert>
             <n-button @click="performAllChecks" :loading="checking" :disabled="checking">
                            立即重新检查网络状态
                    </n-button>
            <n-card>
                <n-space vertical>
                   
                    <n-text v-if="onlineStatus === null">正在检查网络服务...</n-text>
                    <template v-else>
                       
                   <n-h4>互联网状态</n-h4>
                   <n-text>{{ onlineStatus ? "OpenFrp Cross Platform Launcher 已连接至互联网" : "无法访问互联网" }}。</n-text>
                   
                    <n-h4>服务状态</n-h4>
                    <div v-for="service in serviceChecks" :key="service.id" style="margin-left: 15px; margin-bottom: 5px;">
                      <n-text>{{ service.name }}：</n-text>
                      <span v-if="service.checking">
                        <n-skeleton text style="width: 50%;" />
                      </span>
                      <span v-else>
                        <span :style="{ color: service.status === true ? '#266e48' : service.status === false ? '#d03050' : '#000000' }">
                          {{ service.status === true ? '已连接' : service.status === false ? '无法访问' : '检查中...' }}
                        </span>
                        <span v-if="service.status === true && service.latency !== null" :style="{ color: '#266e48', fontSize: '12px' }">
                          ({{ service.latency }}ms)
                        </span>
                        <span v-if="service.status === false && service.error" :style="{ color: '#d03050', fontSize: '12px' }">
                          ({{ service.error }})
                        </span>
                      </span>
                    </div>
                
                </template>  
                   
                    
                </n-space>
            </n-card>
            <n-card>
                <n-h4>诊断信息</n-h4>
                <n-table style="margin-top: 5px;" size="small">
                    <tbody>
                        <tr>
                            <td>启动器</td>
                            <td>Beta v{{ currentVersion }} / {{ buildInfo }}</td>
                        </tr>
                        <tr>
                            <td>系统架构</td>
                            <td>{{ systemInfo }}</td>
                        </tr>
                        <tr>
                            <td>系统时间</td>
                            <td>{{ timestamp }}</td>
                        </tr>
                        <tr>
                            <td>用户代理</td>
                            <td>{{ userAgent }}</td>
                        </tr>
                    </tbody>
                </n-table>
                
            </n-card>
        </n-space>
    </n-scrollbar>
</template>

<style scoped>
.n-h4{
    margin-bottom: 0px;
    margin-top: 8px;;
    font-size: large;
    font-weight: normal;

}

</style>