<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, watch } from 'vue';
import { useDialog, useMessage } from 'naive-ui';
import { useRouter } from 'vue-router';
import { Refresh } from '@vicons/ionicons5';
import copy from 'copy-to-clipboard';
import frpApiGetUserProxies from '@/requests/frpApi/frpApiGetUserProxies';
import frpApiRemoveProxy from '@/requests/frpApi/frpApiRemoveProxy';
import frpApiEditProxy from '@/requests/frpApi/frpApiEditProxy';
import frpApiForceOff from '@/requests/frpApi/frpApiForceOff';
import frpApiChangeProxy from '@/requests/frpApi/frpApiChangeProxy';
import Edit from '@/components/ManageProxies/Edit.vue';
import Infomation from '@/components/ManageProxies/Infomation.vue';
import GetConf from '@/components/ManageProxies/GetConf.vue';
import Menu from '@/components/ManageProxies/Menu.vue';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';
import { hostAndPort, proxyRemoteAddress } from '@/utils/proxies';
import {
  pendingTunnels, rememberTunnel, runningTunnels, startManagedTunnel, stopManagedTunnel,
} from '@/services/tunnelRuntime';

const message = useMessage();
const dialog = useDialog();
const router = useRouter();
const session = useSessionStore();
const proxies = ref<Struct.UserProxy[]>([]);
const search = ref('');
const filter = ref('all');
const loading = ref(false);
const listError = ref('');
const modifying = ref(new Set<number>());
let requestId = 0;
let active = true;

const filterOptions = [
  { label: '全部', value: 'all' },
  ...['TCP', 'UDP', 'HTTP', 'HTTPS', 'STCP', 'XTCP'].map(label => ({ label, value: label.toLowerCase() })),
];
const rows = computed(() => proxies.value.map(proxy => ({
  proxy,
  id: String(proxy.id),
  local: hostAndPort(proxy.localIp, proxy.localPort),
  remote: proxyRemoteAddress(proxy),
})));
type Row = typeof rows.value[number];
const filtered = computed(() => {
  const query = search.value.trim().toLowerCase();
  return rows.value.filter(({ proxy, id, local, remote }) =>
    (filter.value === 'all' || proxy.proxyType.toLowerCase() === filter.value)
    && (!query || [id, proxy.proxyName, local, remote].some(value => value.toLowerCase().includes(query)))
  );
});
const external = computed(() => {
  const ids = new Set(rows.value.map(row => row.id));
  return Array.from(runningTunnels.value.values()).filter(tunnel => tunnel.external && !ids.has(tunnel.id));
});
const colors = { tcp: 'success', udp: 'warning', http: 'info', https: 'primary' } as const;
function typeColor(type: string) {
  return colors[type.toLowerCase() as keyof typeof colors] || 'default';
}

async function fetchList(force = false) {
  if (!session.signedIn || (!force && loading.value)) return;
  const id = ++requestId;
  loading.value = true;
  listError.value = '';
  try {
    const result = await frpApiGetUserProxies();
    if (id !== requestId || !active) return;
    if (!result.flag) throw new Error(result.msg);
    if (!Array.isArray(result.data?.list)) throw new Error('隧道列表格式异常');
    proxies.value = [...result.data.list].sort((a, b) => a.id - b.id);
  } catch (error) {
    if (id === requestId && active) listError.value = errorMessage(error, '获取隧道列表失败');
  } finally {
    if (id === requestId) loading.value = false;
  }
}
watch([() => session.signedIn, () => session.userInfo?.id], ([signedIn, owner], [wasSignedIn, oldOwner]) => {
  if (!signedIn || (oldOwner != null && owner !== oldOwner)) {
    requestId++;
    proxies.value = [];
    loading.value = false;
    listError.value = '';
  }
  if (signedIn && (!wasSignedIn || oldOwner == null || owner !== oldOwner)) void fetchList();
}, { immediate: true });
let refreshTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => { refreshTimer = setInterval(() => void fetchList(), 45000); });
onUnmounted(() => {
  active = false;
  if (refreshTimer) clearInterval(refreshTimer);
});

function copyText(value: string) {
  if (value && copy(value)) message.success('已复制到剪贴板');
  else message.error('复制失败');
}
function disabledReason(row: Row): string {
  if (runningTunnels.value.has(row.id)) return '';
  if (!row.proxy.status) return '隧道已被禁用';
  if (row.proxy.online) return '隧道已在其它设备运行';
  if (!session.userInfo?.token) return '正在获取访问密钥，请稍后重试';
  return '';
}
async function stop(id: string) {
  if (pendingTunnels.has(id)) return;
  try {
    await stopManagedTunnel(id);
    const current = proxies.value.find(proxy => String(proxy.id) === id);
    if (current) current.online = false;
    message.success(`隧道 #${id} 已停止`);
  } catch (error) {
    message.error(`停止隧道失败：${errorMessage(error)}`);
  }
}
async function toggle(row: Row) {
  if (pendingTunnels.has(row.id)) return;
  if (runningTunnels.value.has(row.id)) return stop(row.id);
  const account = session.userInfo;
  if (!account?.token || disabledReason(row)) return;
  try {
    const result = await startManagedTunnel({
      id: row.id, token: account.token, remote: row.remote, name: row.proxy.proxyName, external: false,
    });
    if (result === 'ready' && session.userInfo?.id === account.id) {
      rememberTunnel(account.id, row.id);
      message.success(`隧道 #${row.id} ${row.proxy.proxyName} 启动成功`);
    } else if (result === 'running') {
      message.warning('Frpc 进程已启动，连接结果请查看日志');
    } else if (result === 'already-running') {
      message.info('隧道已在运行');
    }
  } catch (error) {
    message.error(`启动隧道失败：${errorMessage(error)}`);
  }
}

async function mutate(id: number, task: () => Promise<{ flag: boolean; msg: string }>) {
  if (modifying.value.has(id)) return false;
  modifying.value.add(id);
  try {
    const result = await task();
    if (!result.flag) throw new Error(result.msg);
    message.success(result.msg || '操作成功');
    await Promise.all([fetchList(true), session.refreshProfile()]);
    return true;
  } catch (error) {
    message.error(errorMessage(error, '操作失败'));
    return false;
  } finally {
    modifying.value.delete(id);
  }
}

function editProxy(proxy: Struct.UserProxy) {
  const submit = ref(false);
  const window = dialog.info({
    title: `编辑隧道 ${proxy.proxyName}`,
    showIcon: false, maskClosable: false,
    style: { width: 'min(560px, calc(100vw - 32px))' },
    positiveText: '保存', negativeText: '取消',
    content: () => h(Edit, {
      isEditMode: true, editConfig: proxy, watchDog: submit.value,
      async fallback(valid, body) {
        try {
          if (valid && body && await mutate(proxy.id, () => frpApiEditProxy(body))) window.destroy();
        } finally {
          window.loading = false;
          submit.value = false;
        }
      },
    }),
    onPositiveClick: () => {
      if (!submit.value) {
        submit.value = true;
        window.loading = true;
      }
      return false;
    },
    onNegativeClick: () => !submit.value,
  });
}

function selectAction(key: string, row: Row) {
  const proxy = { ...row.proxy, online: row.proxy.online || runningTunnels.value.has(row.id), connectAddress: row.remote };
  if (key === 'refreshState') { void fetchList(); return; }
  if (key === 'stateChange') { void mutate(proxy.id, () => frpApiChangeProxy({ proxy_id: proxy.id, proxy_do: !proxy.status })); return; }
  if (key === 'editConf' && !proxy.online) { editProxy(proxy); return; }
  if (key === 'getInfo') {
    dialog.info({
      title: `隧道 ${proxy.proxyName} 的信息`, showIcon: false,
      content: () => h(Infomation, { proxy, copyFallback: copyText }),
    });
    return;
  }
  if (key === 'getConf') {
    let content = '';
    const window = dialog.info({
      title: `获取配置 · ${proxy.proxyName}`, showIcon: false, positiveText: '复制',
      positiveButtonProps: { disabled: true },
      content: () => h(GetConf, {
        proxy, token: session.userInfo?.token || '',
        fallback(value: string) { content = value; window.positiveButtonProps = { disabled: !value }; },
      }),
      onPositiveClick: () => { copyText(content); return false; },
    });
    return;
  }
  if (key === 'deleteProxy' && !proxy.online) {
    dialog.error({
      title: `移除隧道 ${proxy.proxyName}`, content: '确定要移除该隧道吗？',
      positiveText: '确认', negativeText: '取消',
      onPositiveClick: () => mutate(proxy.id, () => frpApiRemoveProxy({ proxy_id: proxy.id })),
    });
  } else if (key === 'forceOff' && proxy.online && !runningTunnels.value.has(row.id)) {
    dialog.warning({
      title: `强制下线隧道 ${proxy.proxyName}`, content: '确定要下线该隧道吗？部分节点无法远程下线。',
      positiveText: '下线隧道', negativeText: '取消',
      onPositiveClick: () => mutate(proxy.id, () => frpApiForceOff({ proxy_id: proxy.id })),
    });
  }
}
</script>

<template>
  <n-space vertical :size="12">
    <n-h2 style="margin-bottom: 3px;">隧道管理</n-h2>

    <n-card v-if="external.length" title="外部快速启动隧道">
      <n-list bordered hoverable>
        <n-list-item v-for="tunnel in external" :key="tunnel.id">
          <n-space justify="space-between" align="center" style="width: 100%;">
            <n-text strong>外部隧道 #{{ tunnel.id }}</n-text>
            <n-button type="error" size="small" :loading="pendingTunnels.has(tunnel.id)" @click="stop(tunnel.id)">停止</n-button>
          </n-space>
        </n-list-item>
      </n-list>
    </n-card>

    <n-card v-if="!session.signedIn">
      <n-empty description="登录后可管理账户下的隧道">
        <template #extra><n-button type="primary" @click="router.push('/settings')">立即登录</n-button></template>
      </n-empty>
    </n-card>
    <template v-else>
      <n-card>
        <n-space justify="space-between" align="center">
          <n-space align="center" :size="8">
            <n-input v-model:value="search" placeholder="输入 ID、名称或端口筛选" clearable class="tunnel-search" />
            <n-select v-model:value="filter" :options="filterOptions" class="protocol-filter" aria-label="隧道协议" />
            <n-button :loading="loading" @click="fetchList()">
              <template #icon><n-icon><Refresh /></n-icon></template>
              刷新
            </n-button>
          </n-space>
          <n-statistic v-if="session.userInfo" label="隧道（已用 / 总计）" :value="session.userInfo.used">
            <template #suffix>/ {{ session.userInfo.proxies }}</template>
          </n-statistic>
        </n-space>
      </n-card>

      <n-alert v-if="listError" type="warning" title="获取隧道列表失败">{{ listError }}</n-alert>
      <n-skeleton v-if="loading && !rows.length" :height="100" />
      <n-card v-else-if="!filtered.length && !listError">
        <n-empty :description="rows.length ? '没有符合条件的隧道' : '暂无隧道'">
          <template #extra>
            <n-button v-if="rows.length" @click="search = ''; filter = 'all'">清除筛选</n-button>
            <n-button v-else type="primary" @click="router.push('/createproxy')">创建隧道</n-button>
          </template>
        </n-empty>
      </n-card>

      <div v-if="filtered.length" class="tunnel-grid">
        <n-card v-for="row in filtered" :key="row.id" size="small" :title="'隧道 #' + row.id + ' ' + row.proxy.proxyName">
          <template #header-extra>
            <n-space :size="4">
              <n-tag v-if="runningTunnels.get(row.id)?.external" type="info" size="small">快速启动</n-tag>
              <n-tag :type="typeColor(row.proxy.proxyType)" size="small">{{ row.proxy.proxyType.toUpperCase() }}</n-tag>
            </n-space>
          </template>
          <dl class="tunnel-addresses">
            <dt>节点</dt><dd>{{ row.proxy.friendlyNode }}</dd>
            <dt>本地地址</dt><dd>{{ row.local }}</dd>
            <dt>远程地址</dt>
            <dd>
              <button v-if="row.remote" type="button" class="tunnel-address" title="复制远程地址"
                @click="copyText(row.remote)">{{ row.remote }}</button>
              <template v-else>—</template>
            </dd>
          </dl>
          <template #footer>
            <n-space justify="space-between" align="center" :size="8">
              <n-text depth="3">{{ pendingTunnels.has(row.id) ? '处理中…' : runningTunnels.has(row.id) ? '本机运行中' : disabledReason(row) || '已停止' }}</n-text>
              <n-space align="center" :size="12">
                <Menu :proxy="row.proxy" :local-running="runningTunnels.has(row.id)"
                  :busy="pendingTunnels.has(row.id) || modifying.has(row.proxy.id)" :fallback="key => selectAction(key, row)" />
                <n-switch :value="runningTunnels.has(row.id)" :loading="pendingTunnels.has(row.id)"
                  :disabled="!!disabledReason(row) || modifying.has(row.proxy.id)" :aria-label="'启动或停止隧道 ' + row.proxy.proxyName"
                  @update:value="toggle(row)" />
              </n-space>
            </n-space>
          </template>
        </n-card>
      </div>
    </template>
  </n-space>
</template>

<style scoped>
.tunnel-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 300px), 1fr));
  gap: 12px;
}
.tunnel-grid :deep(.n-card-header) { gap: 8px; }
.tunnel-search { width: min(240px, 55vw); }
.protocol-filter { width: 110px; }
.tunnel-addresses {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: baseline;
  gap: 4px 10px;
  margin: 0;
}
.tunnel-addresses dt { color: var(--n-text-color); opacity: 0.7; }
.tunnel-addresses dd { margin: 0; overflow-wrap: anywhere; }
.tunnel-address {
  max-width: 100%;
  padding: 0;
  border: 0;
  color: inherit;
  background: none;
  font: inherit;
  text-align: left;
  white-space: pre-line;
  overflow-wrap: anywhere;
  cursor: pointer;
}
.tunnel-address:hover { text-decoration: underline; }
</style>
