<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useMessage } from 'naive-ui';
import type { FormInst, FormRules } from 'naive-ui';
import { CheckmarkOutline, HelpOutline, ShuffleOutline } from '@vicons/ionicons5';
import { errorMessage } from '@/utils/apiClient';
import { proxyDomains } from '@/utils/proxies';

const props = defineProps<{
  isEditMode?: boolean;
  editConfig?: Struct.UserProxy;
  nodeConfig?: Struct.Node;
  watchDog: boolean;
  fallback: (success: boolean, body?: Struct.EditOrNewUserProxy) => void;
}>();

const protocols = ['tcp', 'udp', 'http', 'https'] as const;
const initial = props.editConfig;
const message = useMessage();
const form = ref<FormInst | null>(null);
const data = ref({
  name: initial?.proxyName || '',
  local_addr: initial?.localIp || '127.0.0.1',
  local_port: initial?.localPort ?? null as number | null,
  remote_port: initial?.remotePort ?? null as number | null,
  domains: proxyDomains(initial?.domain || '').join(','),
  type: initial?.proxyType || protocols.find(type => props.nodeConfig?.protocolSupport?.[type]) || 'tcp',
  dataGzip: initial?.useCompression || false,
  dataEncrypt: initial?.useEncryption || false,
  autoTls: initial?.autoTls || 'false',
  forceHttps: initial?.forceHttps || false,
  proxyProtocolVersion: initial?.proxyProtocolVersion || false,
  custom: initial?.custom || '',
});
const typeOptions = computed(() => {
  if (props.isEditMode) return [{ value: data.value.type, label: data.value.type.toUpperCase() }];
  return protocols.map(type => ({
    value: type, label: type.toUpperCase(), disabled: !props.nodeConfig?.protocolSupport?.[type],
  }));
});
const web = computed(() => ['http', 'https'].includes(data.value.type));
const tlsOptions = [{ label: '关闭', value: 'false' }, { label: '开启', value: 'true' }];
const tlsOptionsWithSelected = computed(() =>
  tlsOptions.some(option => option.value === data.value.autoTls) ? tlsOptions
    : [...tlsOptions, { value: data.value.autoTls, label: data.value.autoTls }]
);
const range = computed<[number, number] | null>(() => {
  const value = props.nodeConfig?.allowPort?.trim();
  if (!value) return null;
  const match = value.match(/^\(\s*(\d+)\s*,\s*(\d+)\s*\)$/);
  if (!match) return null;
  const min = Number(match[1]);
  const max = Number(match[2]);
  return min >= 1 && max <= 65535 && max >= min ? [min, max] : null;
});
const domains = computed(() => data.value.domains.split(/[,，]/).map(value => value.trim()).filter(Boolean));
const isPort = (value: unknown) => typeof value === 'number' && Number.isInteger(value) && value >= 1 && value <= 65535;

const rules: FormRules = {
  name: {
    required: true, trigger: 'blur',
    validator: (_, value: string) => {
      if (initial && value === initial.proxyName) return true;
      return /^[\w.-]{3,22}$/.test(value.trim()) || new Error('请输入 3–22 位字母、数字、下划线、点或连字符');
    },
  },
  local_addr: {
    required: true, trigger: 'blur',
    validator: (_, value: string) => {
      const host = value.trim();
      return (!!host && !/[\s/]/.test(host)) || new Error('请输入有效的本地地址');
    },
  },
  local_port: { required: true, trigger: 'blur', validator: (_, value) =>
    isPort(value) || new Error('请输入 1–65535 之间的本地端口') },
  remote_port: { required: true, trigger: 'blur', validator: (_, value: number) => {
    if (!isPort(value)) return new Error('请输入 1–65535 之间的远程端口');
    const limit = range.value;
    return !limit || (value >= limit[0] && value <= limit[1]) || new Error(`远程端口需在 ${limit[0]}–${limit[1]} 之间`);
  } },
  domains: { required: true, trigger: 'blur', validator: () =>
    (domains.value.length > 0 && domains.value.every(domain => !/[\s/:?#]/.test(domain)))
      || new Error('请输入域名，多个域名请用逗号分隔') },
};

if (!initial) {
  watch(() => data.value.type, (type, previous) => {
    if (type === 'http' && (data.value.local_port === null || (previous === 'https' && data.value.local_port === 443))) {
      data.value.local_port = 80;
    } else if (type === 'https' && (data.value.local_port === null || (previous === 'http' && data.value.local_port === 80))) {
      data.value.local_port = 443;
    }
  }, { immediate: true });
}

function randomName() {
  const letters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  const suffix = Array.from({ length: 7 }, () => letters[Math.floor(Math.random() * letters.length)]).join('');
  data.value.name = `CPL_${suffix}`;
}
function randomPort() {
  const [min, max] = range.value ?? [20000, 65535];
  const excluded = new Set([25565, 19132, 8233, 80, 443]);
  const options = max - min + 1 - [...excluded].filter(port => port >= min && port <= max).length;
  if (options < 1) {
    message.warning('当前节点没有可随机选择的远程端口');
    return;
  }
  let remaining = Math.floor(Math.random() * options);
  for (let port = min; port <= max; port++) {
    if (!excluded.has(port) && remaining-- === 0) {
      data.value.remote_port = port;
      return;
    }
  }
}

watch(() => props.watchDog, async (submit) => {
  if (!submit) return;
  if (!props.isEditMode && !props.nodeConfig?.protocolSupport?.[data.value.type as typeof protocols[number]]) {
    message.warning('请选择节点支持的隧道类型');
    props.fallback(false);
    return;
  }
  const currentForm = form.value;
  if (!currentForm) { props.fallback(false); return; }
  try {
    await currentForm.validate();
  } catch {
    props.fallback(false);
    return;
  }
  const { domains: _, ...values } = data.value;
  const local = values.local_addr.trim();
  props.fallback(true, {
    ...values,
    proxy_id: initial?.id,
    node_id: initial?.nid ?? props.nodeConfig?.id ?? -1,
    name: values.name.trim(),
    local_addr: local.toLowerCase() === 'localhost' ? '127.0.0.1' : local.replace(/^\[(.*)\]$/, '$1'),
    remote_port: web.value && !initial ? -1 : (values.remote_port ?? -1),
    domain_bind: web.value ? JSON.stringify([...new Set(domains.value)]) : (initial?.domain || ''),
  });
});

interface LocalPort { port: number; pid?: number; process?: string }
const portDialog = ref(false);
const portLoading = ref(false);
const portError = ref('');
const ports = ref<(LocalPort & { type: 'TCP' | 'UDP' })[]>([]);
const portSearch = ref('');
const portFilter = ref<'all' | 'TCP' | 'UDP'>('all');
const hideSystemPorts = ref(true);
const selectedPort = ref<string>();
const systemProcess = /^(?:system|svchost|services|idle|init|systemd|launchd|spoolsv|lsass|kernel_task|kthreadd|kworker[^ ]*|cpuhp\/\d+)(?:\.exe)?$/i;
const visiblePorts = computed(() => {
  const query = portSearch.value.trim().toLowerCase();
  const seen = new Set<string>();
  return ports.value.flatMap(entry => {
    const { type, port, pid, process } = entry;
    if (portFilter.value !== 'all' && portFilter.value !== type) return [];
    if (hideSystemPorts.value && process && systemProcess.test(process)) return [];
    if (query && !`${port} ${pid ?? ''} ${process ?? ''} ${type}`.toLowerCase().includes(query)) return [];
    const key = `${type}-${port}-${pid ?? ''}`;
    if (seen.has(key)) return [];
    seen.add(key);
    return [{ ...entry, key }];
  }).sort((a, b) => a.port - b.port);
});
const portChoice = computed(() => visiblePorts.value.find(entry => entry.key === selectedPort.value));

async function loadPorts() {
  if (portLoading.value) return;
  portLoading.value = true;
  portError.value = '';
  selectedPort.value = undefined;
  try {
    const entries = await invoke<{ tcp: LocalPort[]; udp: LocalPort[] }>('get_local_ports');
    ports.value = [
      ...entries.tcp.map(entry => ({ ...entry, type: 'TCP' as const })),
      ...entries.udp.map(entry => ({ ...entry, type: 'UDP' as const })),
    ];
  } catch (error) {
    portError.value = errorMessage(error, '获取本地端口失败');
  } finally {
    portLoading.value = false;
  }
}
function openPortDialog() {
  portSearch.value = '';
  portFilter.value = data.value.type === 'udp' ? 'UDP' : 'TCP';
  portDialog.value = true;
  void loadPorts();
}
function selectPort() {
  if (!portChoice.value) return;
  data.value.local_port = portChoice.value.port;
  portDialog.value = false;
  message.success(`已选择本地端口: ${portChoice.value.port}`);
}
</script>

<template>
  <div class="proxy-editor">
    <n-scrollbar class="proxy-editor-scroll">
      <n-form ref="form" :model="data" :rules="rules" :disabled="watchDog">
        <n-alert v-if="data.local_port === 25565 || data.local_port === 19132" :show-icon="false"
          title="对广大 Minecraft 玩家的温馨提示" type="warning" style="margin-bottom: 12px;">
          公开服务器时，请启用正版验证、登录插件或白名单，保护服务器与玩家账户。
        </n-alert>
        <n-form-item v-if="nodeConfig" label="已选择节点">
          <n-input :value="'#' + nodeConfig.id + ' ' + nodeConfig.name" readonly />
        </n-form-item>
        <div class="proxy-fields">
          <n-form-item path="name" label="隧道名称">
            <n-input v-model:value="data.name" show-count :maxlength="22" placeholder="隧道名称">
              <template #suffix>
                <n-button text aria-label="随机隧道名" @click="randomName"><n-icon :component="ShuffleOutline" /></n-button>
              </template>
            </n-input>
          </n-form-item>
          <n-form-item label="隧道类型">
            <n-select v-model:value="data.type" :options="typeOptions" :disabled="isEditMode || watchDog" />
          </n-form-item>
          <n-form-item path="local_addr" label="本地地址">
            <n-input v-model:value="data.local_addr" placeholder="127.0.0.1">
              <template #suffix>
                <n-tooltip><template #trigger><n-icon :component="HelpOutline" /></template>映射本机服务时可填 127.0.0.1。</n-tooltip>
              </template>
            </n-input>
          </n-form-item>
          <n-form-item path="local_port" label="本地端口">
            <n-input-number v-model:value="data.local_port" :min="1" :max="65535" :precision="0" :show-button="false" placeholder="本地端口">
              <template #suffix><n-button size="tiny" @click="openPortDialog">选择本地端口</n-button></template>
            </n-input-number>
          </n-form-item>
        </div>
        <n-form-item v-if="web" key="domains" path="domains" label="绑定域名">
          <n-input v-model:value="data.domains" placeholder="example.com,example.cn" />
        </n-form-item>
        <n-form-item v-else-if="!isEditMode" key="remote" path="remote_port" label="远程端口">
          <n-input-number v-model:value="data.remote_port" :min="range?.[0] || 1" :max="range?.[1] || 65535" :precision="0"
            :show-button="false" :placeholder="range ? range.join('–') : '远程端口'">
            <template #suffix>
              <n-button text aria-label="随机远程端口" @click="randomPort"><n-icon :component="ShuffleOutline" /></n-button>
            </template>
          </n-input-number>
        </n-form-item>
        <n-collapse>
          <n-collapse-item title="高级配置">
            <div class="proxy-fields">
              <n-form-item label="数据加密"><n-switch v-model:value="data.dataEncrypt" aria-label="数据加密" /></n-form-item>
              <n-form-item label="数据压缩"><n-switch v-model:value="data.dataGzip" aria-label="数据压缩" /></n-form-item>
              <n-form-item v-if="data.type === 'tcp' || data.type === 'https'" label="自动 TLS">
                <n-select v-model:value="data.autoTls" :options="tlsOptionsWithSelected" filterable tag placeholder="选择或输入证书文件名" />
              </n-form-item>
              <n-form-item v-if="data.type === 'https'" label="强制 HTTPS"><n-switch v-model:value="data.forceHttps" aria-label="强制 HTTPS" /></n-form-item>
              <n-form-item label="Proxy Protocol V2"><n-switch v-model:value="data.proxyProtocolVersion" aria-label="Proxy Protocol V2" /></n-form-item>
            </div>
            <n-text v-if="data.type === 'udp'" depth="3">UDP 隧道的 Proxy Protocol V2 仅 OpenFrp 节点支持。</n-text>
            <n-form-item label="自定义配置">
              <n-input v-model:value="data.custom" :autosize="{ minRows: 5, maxRows: 12 }"
                type="textarea" placeholder="一行一项。无效的配置可能导致隧道无法启动。" />
            </n-form-item>
          </n-collapse-item>
        </n-collapse>
      </n-form>
    </n-scrollbar>
    <n-modal v-model:show="portDialog" title="选择本地端口" preset="dialog" class="port-dialog"
      style="width: min(720px, calc(100vw - 32px));">
      <n-space vertical>
        <n-space align="center" :size="8">
          <n-input v-model:value="portSearch" clearable placeholder="搜索端口、进程名或 PID" size="small" />
          <n-radio-group v-model:value="portFilter" size="small">
            <n-radio-button value="all">全部</n-radio-button>
            <n-radio-button value="TCP">TCP</n-radio-button>
            <n-radio-button value="UDP">UDP</n-radio-button>
          </n-radio-group>
          <n-switch v-model:value="hideSystemPorts" aria-label="隐藏系统服务" size="small">
            <template #checked>隐藏系统服务</template>
            <template #unchecked>显示全部端口</template>
          </n-switch>
        </n-space>
        <n-skeleton v-if="portLoading" :height="100" />
        <n-space v-else-if="portError" align="center">
          <n-text type="error">{{ portError }}</n-text>
          <n-button size="small" @click="loadPorts">重试</n-button>
        </n-space>
        <n-empty v-else-if="!visiblePorts.length" description="没有符合条件的监听端口" />
        <n-scrollbar v-else style="max-height: min(340px, 42vh);">
          <div class="port-grid">
            <button v-for="port in visiblePorts" :key="port.key" type="button" class="port-option"
              :aria-pressed="selectedPort === port.key" @click="selectedPort = port.key" @dblclick="selectPort">
              <n-space justify="space-between" :size="4">
                <strong class="port-option-number">
                  {{ port.port }}
                  <n-icon v-if="selectedPort === port.key" :component="CheckmarkOutline" class="port-option-check" aria-hidden="true" />
                </strong>
                <n-tag :type="port.type === 'TCP' ? 'success' : 'warning'" size="small">{{ port.type }}</n-tag>
              </n-space>
              <n-ellipsis>{{ port.process || '未知进程' }}</n-ellipsis>
              <n-text depth="3">PID {{ port.pid ?? '—' }}</n-text>
            </button>
          </div>
        </n-scrollbar>
      </n-space>
      <template #action>
        <n-button @click="portDialog = false">取消</n-button>
        <n-button type="primary" :disabled="!portChoice" @click="selectPort">确定</n-button>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.proxy-editor-scroll { max-height: min(64vh, 600px); }
.proxy-editor :deep(.n-input-number) { width: 100%; }
.proxy-fields {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 210px), 1fr));
  column-gap: 16px;
}
.port-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 140px), 1fr));
  gap: 8px;
  padding: 3px;
}
.port-option {
  min-width: 0;
  display: grid;
  gap: 4px;
  padding: 8px 10px;
  color: inherit;
  background: var(--launcher-surface);
  border: 1px solid transparent;
  border-radius: 5px;
  cursor: pointer;
  text-align: left;
}
.port-option:hover { border-color: var(--launcher-link); }
.port-option[aria-pressed="true"] {
  border-color: var(--launcher-link);
  background: var(--launcher-selection);
}
.port-option:focus-visible { outline: 2px solid var(--launcher-link); outline-offset: 1px; }
.port-option-number { display: inline-flex; align-items: center; gap: 5px; }
.port-option-check { color: var(--launcher-link); }
</style>
