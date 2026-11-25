<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ReloadOutline, CheckmarkCircle, CloseCircle, AlertCircle } from '@vicons/ionicons5';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';
import { OPENFRP_API_BASES } from '@/utils/apiBase';
import frpApiGetNodeList from '@/requests/frpApi/frpApiGetNodeList';
import frpApiGetNodeStatus from '@/requests/frpApi/frpApiGetNodeStatus';

interface ProbeResult {
  success: boolean; status?: number; latency_ms: number; body?: string; error?: string;
}
interface Service {
  name: string; url: string; result?: ProbeResult; pending: boolean;
}
const services = ref<Service[]>([
  ['OPENFRP 基本 API (api.openfrp.net)', `${OPENFRP_API_BASES[0]}/commonQuery/get?key=test`],
  ['OPENFRP 备用 API (ofapi.naids.com)', `${OPENFRP_API_BASES[1]}/commonQuery/get?key=test`],
  ['OpenFrp CPL 更新检查服务', 'https://api.zyghit.cn/updater/ofcpl'],
  ['ZGIT Network OPENAPI', 'https://api.zyghit.cn'],
  ['Natayark OpenID 用户中心', 'https://account.naids.com/api/'],
  ['Natayark 人机验证服务', 'https://cv.naids.com/gen'],
  ['ZGIT Network 基本资源服务', 'https://i.zyghit.cn/'],
  ['ZGIT Network 大型资源文件服务', 'https://r.zyghit.cn/'],
  ['Natayark 静态资源服务', 'https://staticassets.naids.com/'],
].map(([name, url]) => ({ name, url, pending: false })));
const session = useSessionStore();
const userAgent = navigator.userAgent;
const checking = ref(false);
const online = ref<boolean>();
const timestamp = ref('');
const currentVersion = ref('—');
const systemInfo = ref('—');
const buildInfo = ref('—');
const detailedSystemInfo = ref('—');
const proxyBypass = ref<boolean>();
const useDoH = ref<string | null>();
const dohAddr = ref<string | null>();
const forceTls = ref<string | null>();

const ip = ref('');
const location = ref('');
const ipLoading = ref(false);
const fullIP = ref(false);
const maskAddress = (address: string) => address.includes(':')
  ? `${address.split(':').slice(0, 2).filter(Boolean).join(':')}::…`
  : address.replace(/\.\d+$/, '.xxx');
const shownIp = computed(() => fullIP.value ? ip.value : maskAddress(ip.value));

async function probe(url: string, readBody = false): Promise<ProbeResult> {
  try {
    return await invoke<ProbeResult>('probe_url', { url, timeoutMs: 5000, readBody });
  } catch (error) {
    return { success: false, latency_ms: 0, error: errorMessage(error, '网络连接异常') };
  }
}
async function checkIp() {
  ipLoading.value = true;
  fullIP.value = false;
  ip.value = '';
  location.value = '';
  try {
    const result = await probe('https://myip.ipip.net/json', true);
    const value = JSON.parse(result.body || '{}');
    const address: unknown = value.data?.ip;
    if (!result.success || value.ret !== 'ok' || typeof address !== 'string' || !address) return;
    ip.value = address;
    location.value = Array.isArray(value.data.location) ? value.data.location.filter(Boolean).join(' ') : '';
  } catch {
    // 接口异常时保持空值，页面显示获取失败。
  } finally {
    ipLoading.value = false;
  }
}
async function checkServices() {
  const general = probe('https://www.baidu.com');
  await Promise.all(services.value.map(async service => {
    service.pending = true;
    try { service.result = await probe(service.url); }
    finally { service.pending = false; }
  }));
  online.value = (await general).success || services.value.some(service => service.result?.success);
}
function serviceState(result?: ProbeResult) {
  if (!result) return '待检测';
  if (!result.success) return '无法访问';
  const status = result.status ?? 0;
  if (status >= 500) return `服务异常（HTTP ${status}）`;
  if (status >= 400) return `可达（HTTP ${status} · ${result.latency_ms} ms）`;
  return `已连接，延迟 ${result.latency_ms}ms`;
}

type Node = Pick<Struct.Node, 'id' | 'name' | 'port'> & { hostname: string };
const nodes = ref<Node[]>([]);
const nodeCount = ref<number>();
const nodeError = ref('');
const selected = ref<number | null>(null);
const port = ref<number | null>(null);
const testing = ref(false);
const testResult = ref<{ success: boolean; message: string }>();
let nodeRequest = 0;

const nodeOptions = computed(() => nodes.value.map(node => ({ label: `#${node.id} ${node.name}`, value: node.id })));
watch(selected, (id) => {
  port.value = nodes.value.find(node => node.id === id)?.port ?? null;
  testResult.value = undefined;
});
watch(port, () => { testResult.value = undefined; });

async function fetchNodes() {
  const request = ++nodeRequest;
  if (!session.signedIn) {
    nodes.value = [];
    nodeCount.value = undefined;
    selected.value = null;
    return;
  }
  nodeError.value = '';
  const [list, status] = await Promise.all([frpApiGetNodeList(), frpApiGetNodeStatus()]);
  if (request !== nodeRequest) return;
  if (status.flag && Array.isArray(status.data)) {
    nodeCount.value = status.data.filter(node => typeof node?.version === 'string' && node.version.trim()).length;
  } else {
    nodeCount.value = undefined;
  }
  if (!list.flag || !Array.isArray(list.data?.list)) {
    nodeError.value = list.msg || '获取节点列表失败';
    return;
  }
  nodes.value = (list.data.list as Node[]).filter(node => typeof node?.hostname === 'string'
    && /^[\da-z:.[\]_-]+$/i.test(node.hostname) && Number.isInteger(node.port) && node.port > 0 && node.port <= 65535);
  if (!nodes.value.some(node => node.id === selected.value)) selected.value = nodes.value[0]?.id ?? null;
}
const canTest = computed(() => selected.value != null && port.value != null
  && Number.isInteger(port.value) && port.value >= 1 && port.value <= 65535);
async function testNode() {
  if (testing.value || !canTest.value) return;
  const node = nodes.value.find(node => node.id === selected.value);
  if (!node) return;
  const testPort = port.value;
  testing.value = true;
  testResult.value = undefined;
  try {
    const result = await invoke<{ success: boolean; message: string }>('tcp_ping', { host: node.hostname, port: testPort });
    if (node.id === selected.value && testPort === port.value) testResult.value = result;
  } catch (error) {
    if (node.id === selected.value && testPort === port.value) testResult.value = { success: false, message: errorMessage(error) };
  } finally {
    testing.value = false;
  }
}

async function checkEverything() {
  if (checking.value) return;
  checking.value = true;
  online.value = undefined;
  timestamp.value = new Date().toLocaleString();
  await Promise.allSettled([
    checkServices(), checkIp(), fetchNodes(),
    invoke<boolean>('check_proxy_bypass').then(value => { proxyBypass.value = value; }).catch(() => { proxyBypass.value = undefined; }),
    ...([
      ['USE_DOH', useDoH], ['DOH_ADDR', dohAddr], ['FRPC_FORCE_TLS', forceTls],
    ] as const).map(([key, target]) =>
      invoke<string | null>('get_env', { key }).then(value => { target.value = value; }).catch(() => { target.value = undefined; })
    ),
  ]);
  checking.value = false;
}
watch(() => session.signedIn, () => { void fetchNodes(); });
onMounted(() => {
  void checkEverything();
  void Promise.allSettled(([
    ['get_cpl_version', currentVersion], ['get_system_info', systemInfo],
    ['get_build_info', buildInfo], ['get_detailed_system_info', detailedSystemInfo],
  ] as const).map(([command, target]) => invoke<string>(command).then(value => { target.value = value; })));
});
</script>

<template>
  <n-space vertical :size="12">
    <div class="network-toolbar">
      <n-h2 style="margin-bottom: 0;">网络</n-h2>
      <n-button :loading="checking" @click="checkEverything">
        <template #icon><n-icon><ReloadOutline /></n-icon></template>
        重新检查网络状态
      </n-button>
    </div>

    <n-card title="服务状态">
      <n-space vertical :size="6">
        <n-text>{{ online == null ? '正在检查互联网连接…' : online ? 'OpenFrp Cross Platform Launcher 已连接至互联网。' : 'OpenFrp Cross Platform Launcher 无法访问互联网。' }}</n-text>
        <div v-for="service in services" :key="service.url" class="service-row">
          <span class="service-name">
            <n-spin v-if="service.pending" :size="14" />
            <n-icon v-else-if="service.result && (!service.result.success || (service.result.status || 0) >= 500)" color="#d03050"><CloseCircle /></n-icon>
            <n-icon v-else-if="service.result && ((service.result.status || 0) >= 400 || service.result.latency_ms > 500)" color="#c68a25"><AlertCircle /></n-icon>
            <n-icon v-else-if="service.result" color="#18a058"><CheckmarkCircle /></n-icon>
            {{ service.name }}
          </span>
          <n-text depth="3" :title="service.result?.error">{{ service.pending ? '测试中…' : serviceState(service.result) }}</n-text>
        </div>
        <n-text v-if="session.signedIn" depth="3">在线节点数：{{ nodeCount == null ? '暂不可用' : nodeCount + ' 个' }}</n-text>
      </n-space>
    </n-card>

    <n-card v-if="session.signedIn" title="节点连通性测试">
      <n-space vertical>
        <n-alert v-if="nodeError" type="warning">{{ nodeError }}</n-alert>
        <n-form @submit.prevent="testNode">
          <n-form-item label="选择节点">
            <n-select v-model:value="selected" :options="nodeOptions" :disabled="testing || !nodes.length" filterable placeholder="请选择一个节点" />
          </n-form-item>
          <n-form-item label="端口">
            <div class="test-controls">
              <n-input-number v-model:value="port" :min="1" :max="65535" :precision="0" :disabled="testing" />
              <n-button attr-type="submit" :loading="testing" :disabled="!canTest">测试连接</n-button>
            </div>
          </n-form-item>
        </n-form>
        <n-alert v-if="testResult" :type="testResult.success ? 'success' : 'error'"
          :title="testResult.success ? '测试成功' : '测试失败'" closable @close="testResult = undefined">
          {{ testResult.message }}
        </n-alert>
      </n-space>
    </n-card>

    <n-card title="诊断信息">
      <n-table size="small" class="diagnostics-table">
        <tbody>
          <tr><th scope="row">启动器版本</th><td>Beta v{{ currentVersion }} / {{ buildInfo }}</td></tr>
          <tr><th scope="row">系统架构</th><td>{{ systemInfo }}</td></tr>
          <tr><th scope="row">系统版本</th><td>{{ detailedSystemInfo }}</td></tr>
          <tr><th scope="row">系统时间</th><td>{{ timestamp }}</td></tr>
          <tr><th scope="row">用户代理</th><td>{{ userAgent }}</td></tr>
          <tr>
            <th scope="row">IP 信息</th>
            <td>
              <n-text v-if="ipLoading">获取中…</n-text>
              <n-space v-else-if="ip" align="center" :size="8">
                <span>{{ shownIp }} {{ location }}</span>
                <n-button text type="primary" @click="fullIP = !fullIP">{{ fullIP ? '隐藏完整 IP' : '显示完整 IP' }}</n-button>
              </n-space>
              <n-text v-else depth="3">获取失败</n-text>
            </td>
          </tr>
          <tr><th scope="row">代理状态</th><td>{{ proxyBypass == null ? '未知' : proxyBypass ? '已绕过系统代理' : '正在使用系统代理' }}</td></tr>
          <tr><th scope="row">网络配置</th>
            <td>
              DoH {{ useDoH === undefined ? '未知' : useDoH === 'true' ? '已启用（' + (dohAddr || '223.5.5.5') + '）' : '未启用' }}
              · Force TLS {{ forceTls === undefined ? '未知' : forceTls === 'true' ? '已启用' : '未启用' }}
            </td>
          </tr>
        </tbody>
      </n-table>
    </n-card>
  </n-space>
</template>

<style scoped>
.network-toolbar, .test-controls {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
}
.test-controls { justify-content: flex-start; }
.service-row {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  gap: 4px 16px;
  padding: 5px 0;
  border-bottom: 1px solid var(--n-border-color);
}
.service-name { display: inline-flex; align-items: center; gap: 6px; min-width: 0; }
.service-name :deep(.n-icon) { flex-shrink: 0; }
.diagnostics-table th { width: 112px; }
.diagnostics-table td { overflow-wrap: anywhere; }
</style>
