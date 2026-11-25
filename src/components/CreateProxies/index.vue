<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useMessage } from 'naive-ui';
import { openUrl } from '@tauri-apps/plugin-opener';
import { RefreshOutline } from '@vicons/ionicons5';
import frpApiGetNodeList from '@/requests/frpApi/frpApiGetNodeList';
import frpApiNewProxy from '@/requests/frpApi/frpApiNewProxy';
import { useSessionStore } from '@/stores/session';
import { errorMessage } from '@/utils/apiClient';
import { groupAllowsNode } from '@/utils/proxies';
import Edit from '../ManageProxies/Edit.vue';
import Card from './Card.vue';
import './style.less';

const router = useRouter();
const message = useMessage();
const session = useSessionStore();
const nodes = ref<Struct.Node[]>([]);
const loading = ref(false);
const failure = ref('');
const filter = ref('all');
const showFull = ref(true);
const showRestricted = ref(true);
const selectedNode = ref<Struct.Node>();
const showEditor = ref(false);
const submitting = ref(false);
let requestId = 0;
let active = true;

const filterOptions = [
  { label: '全部协议', value: 'all' },
  { label: '支持 UDP', value: 'udp' },
  { label: '支持 HTTP(S)', value: 'web' },
  { label: '大带宽（≥ 50 Mbps）', value: 'bandwidth' },
];
const areas = [
  { id: 1, title: '中国大陆', note: '均需实名', tone: 'info' },
  { id: 2, title: '中国香港 / 中国台湾 / 中国澳门', note: '免备建站', tone: 'success' },
  { id: 3, title: '海外', note: '不建议中国大陆使用', tone: 'warning' },
] as const;

function restriction(node: Struct.Node) {
  const user = session.userInfo;
  if (node.status !== 200) return '节点暂不可用';
  if (node.fullyLoaded) return '节点已满载';
  if (!user) return '正在获取用户信息';
  if (!groupAllowsNode(user.group, node)) return '当前用户组无权限';
  if ((node.classify === 1 || node.needRealname) && !user.realname) return '待实名认证';
  if (!node.protocolSupport || !Object.values(node.protocolSupport).some(Boolean)) return '节点暂不支持可用协议';
  return '';
}
const filtered = computed(() => {
  const account = session.userInfo;
  return nodes.value.filter(node =>
    (showFull.value || !node.fullyLoaded)
    && (showRestricted.value || !account || groupAllowsNode(account.group, node))
    && (filter.value === 'all'
      || filter.value === 'udp' && node.protocolSupport?.udp
      || filter.value === 'web' && (node.protocolSupport?.http || node.protocolSupport?.https)
      || filter.value === 'bandwidth' && node.bandwidth >= 50)
  );
});
const areasWithNodes = computed(() => {
  const groups = areas.map(area => ({
    ...area, nodes: filtered.value.filter(node => node.classify === area.id),
  }));
  const other = filtered.value.filter(node => !areas.some(area => area.id === node.classify));
  return other.length ? [...groups, { id: 0, title: '其他', note: '', tone: 'info' as const, nodes: other }] : groups;
});

async function getNodeList() {
  if (loading.value || !session.signedIn) return;
  const id = ++requestId;
  loading.value = true;
  failure.value = '';
  try {
    const response = await frpApiGetNodeList();
    if (id !== requestId || !active) return;
    if (!response.flag) throw new Error(response.msg);
    if (!Array.isArray(response.data.list)) throw new Error('节点列表格式异常');
    nodes.value = response.data.list;
  } catch (error) {
    if (id === requestId && active) failure.value = errorMessage(error, '获取节点列表失败');
  } finally {
    if (id === requestId) loading.value = false;
  }
}
watch(() => session.signedIn, (signedIn) => {
  if (signedIn) void getNodeList();
  else {
    requestId++;
    nodes.value = [];
    loading.value = false;
    showEditor.value = false;
  }
}, { immediate: true });
onUnmounted(() => { active = false; });

function selectNode(node: Struct.Node) {
  const reason = restriction(node);
  if (reason) {
    message.warning(reason);
    return;
  }
  selectedNode.value = node;
  showEditor.value = true;
}
async function save(valid: boolean, body?: Struct.EditOrNewUserProxy) {
  try {
    if (!valid || !body) return;
    const response = await frpApiNewProxy(body);
    if (!response.flag) throw new Error(response.msg);
    message.success(response.msg || '创建成功');
    showEditor.value = false;
    void session.refreshProfile();
    if (active) await router.push('/proxylist');
  } catch (error) {
    message.error(errorMessage(error, '创建隧道失败'));
  } finally {
    submitting.value = false;
  }
}
function openUsercenter() {
  void openUrl('https://console.openfrp.net/usercenter').catch(() => message.error('打开个人中心失败'));
}
</script>

<template>
  <n-space vertical :size="12" class="createchannel">
    <n-h2 style="margin-bottom: 3px;">创建隧道</n-h2>
    <n-card v-if="!session.signedIn">
      <n-empty description="登录后可创建隧道">
        <template #extra><n-button type="primary" @click="router.push('/settings')">立即登录</n-button></template>
      </n-empty>
    </n-card>
    <template v-else>
      <n-alert v-if="session.userInfo && !session.userInfo.realname" type="warning" title="中国大陆节点需要实名认证">
        <n-button text type="primary" @click="openUsercenter">前往个人中心实名认证</n-button>
      </n-alert>
      <n-alert v-else-if="!session.userInfo && session.profileError" type="warning" title="获取用户信息失败">
        <n-space align="center">
          {{ session.profileError }}
          <n-button size="small" :loading="session.loading" @click="session.refreshProfile">重试</n-button>
        </n-space>
      </n-alert>
      <n-space align="center">
        <n-button tertiary :loading="loading" @click="getNodeList">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          刷新
        </n-button>
        <n-select v-model:value="filter" :options="filterOptions" style="width: min(190px, 52vw);" aria-label="节点筛选" />
        <n-checkbox v-model:checked="showFull">显示满载节点</n-checkbox>
        <n-checkbox v-model:checked="showRestricted">显示无权限节点</n-checkbox>
      </n-space>
      <n-alert v-if="failure" type="error" title="获取节点列表失败">{{ failure }}</n-alert>
      <n-skeleton v-if="loading && !nodes.length" :height="120" />
      <n-card v-else-if="!filtered.length && !failure">
        <n-empty :description="nodes.length ? '没有符合条件的节点' : '暂无节点'" />
      </n-card>
      <n-collapse v-if="filtered.length" default-expanded-names="1">
        <n-collapse-item v-for="area in areasWithNodes" :key="area.id" :name="String(area.id)">
          <template #header>
            <n-space align="center" :size="8">
              <n-text>{{ area.title }}</n-text>
              <n-gradient-text v-if="area.note" :type="area.tone">{{ area.note }}</n-gradient-text>
              <n-text depth="3">{{ area.nodes.length }} 个节点</n-text>
            </n-space>
          </template>
          <div v-if="area.nodes.length" class="node-grid">
            <Card v-for="node in area.nodes" :key="node.id" :node="node" :reason="restriction(node)" :fallback="selectNode" />
          </div>
          <n-empty v-else description="没有符合条件的节点" />
        </n-collapse-item>
      </n-collapse>
    </template>
  </n-space>
  <n-modal v-model:show="showEditor" preset="dialog" title="创建隧道" style="width: min(560px, calc(100vw - 32px));"
    :mask-closable="!submitting" :close-on-esc="!submitting" :closable="!submitting"
    :positive-button-props="{ loading: submitting, disabled: submitting }"
    :negative-button-props="{ disabled: submitting }" positive-text="创建" negative-text="取消"
    :on-positive-click="() => { submitting = true; return false }">
    <Edit v-if="showEditor && selectedNode" :key="selectedNode.id" :node-config="selectedNode" :watch-dog="submitting" :fallback="save" />
  </n-modal>
</template>
