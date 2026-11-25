<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import frpApiGetNodeConf from '@/requests/frpApi/frpApiGetNodeConf';
import { errorMessage } from '@/utils/apiClient';

const props = defineProps<{
  proxy: Struct.UserProxy;
  token: string;
  fallback: (value: string) => void;
}>();
const active = ref('re');
const config = ref('');
const loading = ref(false);
const error = ref('');
const command = computed(() => props.token ? `./frpc -u ${props.token} -p ${props.proxy.id}` : '');

async function loadConfig() {
  loading.value = true;
  error.value = '';
  try {
    const result = await frpApiGetNodeConf({ node_id: props.proxy.nid });
    if (!result.flag) throw new Error(result.msg);
    if (typeof result.data !== 'string' || !result.data.trim()) throw new Error('节点未返回配置文件');
    const [base, ...blocks] = result.data.replace(/\r\n/g, '\n').split(/^\[\[proxies\]\][ \t]*$/m);
    const name = JSON.stringify(props.proxy.proxyName);
    const own = blocks.filter(block => block.split('\n').some(line => line.trim() === `name = ${name}`));
    if (!own.length) throw new Error('配置文件中未找到此隧道');
    config.value = base + own.map(block => `[[proxies]]${block}`).join('');
    if (active.value === 'legacy') props.fallback(config.value);
  } catch (cause) {
    error.value = errorMessage(cause, '获取配置失败');
  } finally {
    loading.value = false;
  }
}
function select(value: string | string[] | null) {
  active.value = Array.isArray(value) ? (value[0] || '') : (value || '');
  props.fallback(active.value === 're' ? command.value : active.value === 'legacy' ? config.value : '');
  if (active.value === 'legacy' && !config.value && !loading.value && !error.value) void loadConfig();
}
onMounted(() => props.fallback(command.value));
</script>

<template>
  <n-collapse accordion default-expanded-names="re" @update:expanded-names="select">
    <n-collapse-item name="re" title="简易启动">
      <n-alert title="安全性警告" type="warning">请妥善保管访问密钥。如有泄露，请前往个人中心重置。</n-alert>
      <n-code v-if="command" word-wrap language="shell" :code="command" />
      <n-text v-else depth="3">未获取到访问密钥</n-text>
    </n-collapse-item>
    <n-collapse-item name="legacy" title="配置文件">
      <n-skeleton v-if="loading" :height="80" />
      <n-space v-else-if="error" align="center">
        <n-text type="error">{{ error }}</n-text>
        <n-button size="small" @click="loadConfig">重试</n-button>
      </n-space>
      <n-scrollbar v-else style="max-height: 48vh;">
        <n-code language="toml" :code="config" word-wrap />
      </n-scrollbar>
    </n-collapse-item>
  </n-collapse>
</template>
