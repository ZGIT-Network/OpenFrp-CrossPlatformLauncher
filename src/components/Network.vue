<script setup lang="ts">
import { NCard, NAlert, NSpace, NH4, NH2, NList, NIcon, NThing, NTable, NScrollbar, NImage, NGradientText, NText, NConfigProvider, NButton, NTag, NResult, NSkeleton, NSelect, NFormItem, NForm, NInputNumber } from 'naive-ui';
import { ref, onMounted, computed, Ref, inject } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ReloadOutline } from '@vicons/ionicons5';

const currentVersion = ref('v0.1')

import frpApiGetNodeStatus from '@/requests/frpApi/frpApiGetNodeStatus';
import frpApiGetNodeList from '@/requests/frpApi/frpApiGetNodeList';

const userInfoObj = inject<{ userInfo: Ref<Struct.UserInfo | undefined>, getUserInfo: () => void }>('userInfo');

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

const isLoggedIn = computed(() => {
    return !!userInfoObj?.userInfo?.value?.token;
});

// 节点测试相关
interface NodeItem {
    id: number;
    name: string;
    hostname: string;
    port: number;
    status: number;
    description: string;
}

const nodeList = ref<NodeItem[]>([]);
const selectedNode = ref<number | null>(null);
const testPort = ref<number>(8120);
const nodeTesting = ref(false);
const nodeTestResult = ref<{
    success: boolean;
    message: string;
    latency?: any;
} | null>(null);

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
        name: 'OPENFRP 基本 API  (ofapi)',
        url: 'https://of-dev-api.bfsea.com/commonQuery/get?key=test',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'backup-api',
        name: 'OPENFRP 备用 API  (bfsea)',
        url: 'https://of-dev-api.bfsea.com/commonQuery/get?key=test',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'cplupd',
        name: 'OpenFrp CPL 更新检查服务',
        url: 'https://api.zyghit.cn/updater/ofcpl',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'zgit-api',
        name: 'ZGIT Network OPENAPI',
        url: 'https://api.zyghit.cn',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    
    {
        id: 'nykid',
        name: 'Natayark OpenID 用户中心',
        url: 'https://account.naids.com/api/',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'collect-api',
        name: 'Natayark 人机验证服务',
        url: 'https://captcha.naids.com/gen',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'i-zyghit',
        name: 'ZGIT Network 基本资源服务',
        url: 'https://i.zyghit.cn/',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'r-zyghit',
        name: 'ZGIT Network 大型资源文件服务',
        url: 'https://r.zyghit.cn/',
        status: null,
        checking: false,
        error: null,
        latency: null
    },
    {
        id: 'r-naids',
        name: 'Natayark 静态资源服务',
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
const ipInfo = ref('')
const showFullIP = ref(false)

// 代理网络检测相关
const proxyBypassStatus = ref<boolean | null>(null)
const useDoHStatus = ref<boolean | null>(null)
const dohAddr = ref<string | null>(null)
const frpcForceTlsEnabled = ref<boolean | null>(null)
const proxyNetworkTestResults = ref<any>(null)
const testingProxyNetwork = ref<boolean>(false)

const detailedSystemInfo = ref('')
const all_total_node = ref(0);


const getNodeStatus = () => {
    frpApiGetNodeStatus().then((res) => {
        if (res.flag)
            all_total_node.value = 0;

        // 将每个节点的 total_traffic_in 累加到 all_total_traffic_in
        res.data.forEach((item: any) => {
            if (item.version !== '') all_total_node.value += 1;
        });

        // console.log(res.data);

    }
    );
};

getNodeStatus();


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

// 检查代理绕过状态
const checkProxyBypassStatus = async () => {
    try {
        const isBypassed = await invoke('check_proxy_bypass') as boolean
        proxyBypassStatus.value = isBypassed
    } catch (e) {
        console.error('检查代理状态失败:', e)
        proxyBypassStatus.value = null
    }
}   

const checkDoHStatus = async () => {
    try {
        const useDohEnv = await invoke('get_env', { key: 'USE_DOH' }) as (string | null)
        useDoHStatus.value = (useDohEnv === 'true')
        if (useDoHStatus.value) {
            // 获取自定义 DoH 地址（若未设置则保持默认）
            const addr = await invoke('get_env', { key: 'DOH_ADDR' }) as (string | null)
            dohAddr.value = addr && addr.length > 0 ? addr : '223.5.5.5'
        } else {
            dohAddr.value = null
        }
    } catch (e) {
        console.error('检查 DoH 状态失败:', e)
        useDoHStatus.value = null
        dohAddr.value = null
    }
}

const checkForceTlsStatus = async () => {
    try {
        const forceTlsEnv = await invoke('get_env', { key: 'FRPC_FORCE_TLS' }) as (string | null)
        frpcForceTlsEnabled.value = (forceTlsEnv === 'true')
    } catch (e) {
        console.error('检查 Force TLS 状态失败:', e)
        frpcForceTlsEnabled.value = null
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

        // 只有响应成功或者403/404状态码视为正常连接
        service.status = response.ok || response.status === 403 || response.status === 404
        if (!response.ok && response.status !== 403 && response.status !== 404) {
            service.error = `HTTP ${response.status}: ${response.statusText}`
        }
        clearTimeout(timeoutId)
    } catch (e: any) {
        console.error(`${service.name}检查失败:`, e)
        // 检查是否为CORS错误
        if (e.name === 'TypeError' && (e.message.includes('CORS') || e.message.includes('Failed to fetch'))) {
            service.status = false  // CORS错误应视为连接失败
            service.error = 'CORS限制或网络问题'
        } else {
            service.status = false
            service.error = e.message || '网络连接异常'
        }
    } finally {
        service.checking = false
    }
}


const getIpInfo = async () => {
    try {
        const response = await fetch('https://myip.ipip.net/json', {
            mode: 'cors',
            headers: {
                'Accept': 'application/json'
            }
        })
        if (response.ok) {
            const data = await response.json()
            ipInfo.value = JSON.stringify(data)
        } else {
            ipInfo.value = '获取失败'
        }
    } catch (e) {
        console.error('获取IP信息失败:', e)
        ipInfo.value = '获取失败'
    }
}

// 获取解析后的IP信息
const parsedIpInfo = computed(() => {
    try {
        return JSON.parse(ipInfo.value)
    } catch (e) {
        return null
    }
})

// 获取打码后的IP
const maskedIP = computed(() => {
    if (!parsedIpInfo.value || parsedIpInfo.value.ret !== 'ok') return '获取失败'
    const ip = parsedIpInfo.value.data.ip
    const parts = ip.split('.')
    if (parts.length !== 4) return ip
    return `${parts[0]}.${parts[1]}.xxx.${parts[3]}`
})

// 获取完整IP
const fullIP = computed(() => {
    if (!parsedIpInfo.value || parsedIpInfo.value.ret !== 'ok') return '获取失败'
    return parsedIpInfo.value.data.ip
})

// 获取位置信息
const locationInfo = computed(() => {
    if (!parsedIpInfo.value || parsedIpInfo.value.ret !== 'ok') return ''
    return parsedIpInfo.value.data.location.join(' ')
})

// 切换IP显示状态
const toggleIPDisplay = () => {
    showFullIP.value = !showFullIP.value
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
        await checkProxyBypassStatus()
        await checkDoHStatus()
        await checkForceTlsStatus()
        // 增加延迟，使检查过程更明显
        await new Promise(resolve => setTimeout(resolve, 300))
        await checkAllServicesSequentially()
        // 获取节点列表
        await getNodeList()

    } finally {
        checking.value = false
    }
}

onMounted(async () => {
    try {
        currentVersion.value = await invoke('get_cpl_version')
        systemInfo.value = await invoke('get_system_info')
        buildInfo.value = await invoke('get_build_info')
        detailedSystemInfo.value = await invoke('get_detailed_system_info')

        await getIpInfo()
    } catch (e) {
        console.error('获取版本信息失败:', e)
    }

    // 组件挂载后自动执行检查
    performAllChecks()
})
const timestamp = new Date().toISOString()
const userAgent = navigator.userAgent


// 获取节点列表
const getNodeList = async () => {
    try {
        const res = await frpApiGetNodeList();
        if (res.flag && res.data && res.data.list) {
            nodeList.value = res.data.list.map((item: any) => ({
                id: item.id,
                name: item.name,
                hostname: item.hostname,
                port: item.port,
                status: item.status,
                description: item.description
            }));

            // 如果还没有选择节点，默认选择第一个
            if (!selectedNode.value && nodeList.value.length > 0) {
                selectedNode.value = nodeList.value[0].id;
                testPort.value = nodeList.value[0].port;
            }
        }
    } catch (e) {
        console.error('获取节点列表失败:', e);
    }
};

// 节点选择改变时更新端口
const onNodeChange = (value: number) => {
    const node = nodeList.value.find(n => n.id === value);
    if (node) {
        testPort.value = node.port;
    }
};

interface TcpPingResult {
    success: boolean;
    message: string;
    latency_ms: number | null;
}

// 测试节点连通性（通过后端实现）
const testNodeConnectivity = async () => {
    if (!selectedNode.value) {
        nodeTestResult.value = {
            success: false,
            message: '请选择一个节点'
        };
        return;
    }

    const node = nodeList.value.find(n => n.id === selectedNode.value);
    if (!node) {
        nodeTestResult.value = {
            success: false,
            message: '未找到选择的节点'
        };
        return;
    }

    nodeTesting.value = true;
    nodeTestResult.value = null;

    try {
        // 调用后端的TCP ping功能
        const result = await invoke<TcpPingResult>('tcp_ping', {
            host: node.hostname,
            port: testPort.value
        });

        nodeTestResult.value = {
            success: result.success,
            message: result.message,
            latency: result.latency_ms
        };
    } catch (e) {
        nodeTestResult.value = {
            success: false,
            message: '测试过程中发生错误: ' + (e as Error).message
        };
    } finally {
        nodeTesting.value = false;
    }
};


// 节点选择选项
// 节点选择选项
const nodeOptions = computed(() => {
  return nodeList.value.map(node => ({
    label: `#${node.id} ${node.name} (${maskHostname(node.hostname)})`,
    value: node.id
  }));
});

// 添加函数用于打码hostname
const maskHostname = (hostname: string): string => {
  
  // 按照域名规则处理，将域名分割成各部分
  const parts = hostname.split('.');
  
  // 如果是三级或更高级域名（至少3个部分）
  if (parts.length >= 3) {
    // 对子域名部分（第一个部分）进行打码
    const subdomain = parts[0];
    let maskedSubdomain;
    
    if (subdomain.length <= 4) {
      // 如果子域名很短，只显示首尾字符
      maskedSubdomain = subdomain.charAt(0) + "***" + subdomain.charAt(subdomain.length - 1);
    } else {
      // 保留前2位和后2位，中间用***代替
      maskedSubdomain = subdomain.substring(0, 5) + "***" + subdomain.substring(subdomain.length - 5);
    }
    
    // 重新组合域名
    parts[0] = maskedSubdomain;
    return parts.join('.');
  }
  
  // 对于其他域名格式，保留前3位和后3位
  if (hostname.length <= 8) {
    // 如果域名很短，只显示首尾字符
    return hostname.charAt(0) + "***" + hostname.charAt(hostname.length - 3);
  } else {
    // 保留前3位和后3位
    const prefix = hostname.substring(0, 3);
    const suffix = hostname.substring(hostname.length - 3);
    return prefix + "***" + suffix;
  }
};
</script>
<template>
    <n-scrollbar>
        <n-h2 style="margin-bottom: 3px;">网络</n-h2>

        <n-space vertical>


            <n-card>
                <n-space vertical>

                    <n-text v-if="onlineStatus === null">正在检查网络服务...</n-text>
                    <template v-else>

                        <n-h4>互联网状态</n-h4>
                        <n-text style="margin-left: 15px; margin-bottom: 5px;">{{ onlineStatus ? "OpenFrp Cross Platform Launcher 已连接至互联网" : "OpenFrp Cross Platform Launcher 无法访问互联网" }}。</n-text>

                        <n-h4>服务状态</n-h4>
                        <div v-for="service in serviceChecks" :key="service.id"
                            style="margin-left: 15px; margin-bottom: 5px;">
                            <n-text>{{ service.name }}：</n-text>
                            <span v-if="service.checking">
                                <n-skeleton text style="width: 50%;" />
                            </span>
                            <span v-else>
                                <span
                                    :style="{ color: service.status === true ? '#266e48' : service.status === false ? '#d03050' : '#000000' }">
                                    {{ service.status === true ? '已连接' : service.status === false ? '无法访问' : '检查中...' }}
                                </span>
                                <span v-if="service.status === true && service.latency !== null"
                                    :style="{ color: '#266e48', fontSize: '12px' }">
                                    ({{ service.latency }}ms)
                                </span>
                                <span v-if="service.status === false && service.error"
                                    :style="{ color: '#d03050', fontSize: '12px' }">
                                    ({{ service.error }})
                                </span>
                            </span>
                        </div>

                        <template v-if="isLoggedIn">

                        <n-h4>节点数据</n-h4>
                        <n-text style="margin-left: 15px; margin-bottom: 5px;"><span>在线节点数：</span>{{ all_total_node }}
                            个</n-text>
                        </template>
                        <n-text v-else>尚未登录到启动器。</n-text>

                    </template>


                </n-space>
            </n-card>
            <!-- 节点连通性测试卡片 -->
             <n-card v-if="isLoggedIn">
                <n-space vertical>
                    <n-h4>节点连通性测试</n-h4>
                    <n-text style="margin-bottom: 10px;">选择一个节点并测试连接延迟和可达性 <br/><small>*请注意，如果测试结果始终为 正常（0ms）,请关闭您的网络代理</small></n-text>

                    <n-form>
                        <n-form-item label="选择节点" path="selectedNode" style="margin-bottom: -15px;">
                            <n-select
                                v-model:value="selectedNode"
                                :options="nodeOptions"
                                placeholder="请选择一个节点"
                                @update:value="onNodeChange"
                            />
                        </n-form-item>

                        <n-form-item label="端口">
                            <!-- <n-slider v-model:value="testPort" :step="1" :max="65535" /> -->
                            <n-input-number
                                v-model:value="testPort"
                                :min="1"
                                :max="65535"
                                style="width: 70%"
                            /> 
                            <n-button
                            @click="testNodeConnectivity"
                            :loading="nodeTesting"
                            :disabled="nodeTesting || !selectedNode"
                            style="margin-left:8px;width: 30%"
                            type="default"
                        >
                            {{ nodeTesting ? '测试中...' : '测试连接' }}
                        </n-button>
                        </n-form-item>

                        
                    </n-form>

                    <div v-if="nodeTestResult" style="margin-top: -15px;">
                        <n-alert
                            :type="nodeTestResult.success ? 'success' : 'error'"
                            :title="nodeTestResult.success ? '测试成功' : '测试失败'"
                            closable
                        >
                            <n-space vertical :size="4">
                                <n-text>{{ nodeTestResult.message }}</n-text>
                            </n-space>
                        </n-alert>
                    </div>
                </n-space>
            </n-card>
            <n-card>
                <n-h4>诊断信息</n-h4>
                <n-table style="margin-top: 5px;" size="small">
                    <tbody>
                        <tr>
                            <td>启动器版本</td>
                            <td>Beta v{{ currentVersion }} / {{ buildInfo }}</td>
                        </tr>
                        <tr>
                            <td>系统架构</td>
                            <td>{{ systemInfo }}</td>
                        </tr>
                        <tr>
                            <td>系统版本</td>
                            <td>{{ detailedSystemInfo }}</td>
                        </tr>
                        <tr>
                            <td>系统时间</td>
                            <td>{{ timestamp }}</td>
                        </tr>
                        <tr>
                            <td>用户代理</td>
                            <td>{{ userAgent }}</td>
                        </tr>
                        <tr>
                            <td>IP信息</td>
                            <td>
                                <n-text v-if="!parsedIpInfo || parsedIpInfo.ret !== 'ok'">{{ ipInfo }}</n-text>
                                <n-text v-else>
                                    <span @click="toggleIPDisplay" style="cursor: pointer; color: #2080f0;">
                                        {{ showFullIP ? fullIP : maskedIP }} ({{ locationInfo }})
                                    </span>
                                </n-text>
                            </td>
                        </tr>
                        <tr>
                            <td>代理状态</td>
                            <td>
                               
                                <span >
                                    <span :style="{ color: proxyBypassStatus ? '#266e48' : '' }">
                                        {{ proxyBypassStatus ? '已绕过系统代理' : '正在使用系统代理' }}
                                    </span>
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>网络配置</td>
                            <td>
                               
                                <span >
                                    <span :style="{ color: useDoHStatus ? '#2080f0' : '', marginRight: '10px' }">
                                        {{ useDoHStatus ? '已启用 DoH (' + dohAddr + ')' : '未启用 DoH' }}
                                    </span>
                                    <span :style="{ color: frpcForceTlsEnabled ? '#2080f0' : '' }">
                                        {{ frpcForceTlsEnabled ? '已启用 Force TLS' : '未启用 Force TLS' }}
                                    </span>
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </n-table>
            

            </n-card>

            <n-button @click="performAllChecks" :loading="checking" :disabled="checking" style="width: 100%;">
                <template #icon>
                    <NIcon>
                        <ReloadOutline />
                    </NIcon>
                </template>
                重新检查网络状态
            </n-button>

        </n-space>
    </n-scrollbar>
</template>

<style scoped>
.n-h4 {
    margin-bottom: 0px;
    margin-top: 8px;
    ;
    font-size: large;
    font-weight: normal;

}
</style>