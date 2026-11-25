<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useDialog, useMessage } from 'naive-ui';
import type { LogInst } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { useRoute } from 'vue-router';
import { globalLogService } from '@/services/logService';

const route = useRoute();
const message = useMessage();
const dialog = useDialog();
const log = ref<LogInst | null>(null);
const autoScroll = ref(true);
const selected = ref('all');
const rows = ref(15);
const logEntries = globalLogService.entries;
const logOptions = computed(() => {
  const ids = new Set(logEntries.value.flatMap(entry => entry.tunnel ? [entry.tunnel] : []));
  if (selected.value !== 'all') ids.add(selected.value);
  return [{ label: '全部日志', value: 'all' }, ...Array.from(ids).map(id => ({ label: `隧道 #${id}`, value: id }))];
});
const entries = computed(() => selected.value === 'all'
  ? logEntries.value : logEntries.value.filter(entry => entry.tunnel === selected.value));
const output = computed(() => entries.value.map(entry => entry.text).join('\n'));
const version = ref('');
const systemInfo = ref('');
const buildInfo = ref('');

watch(() => route.query.tunnel, (tunnel) => {
  selected.value = typeof tunnel === 'string' && /^(?:\d+|link-[\w-]+)$/.test(tunnel) ? tunnel : 'all';
}, { immediate: true });
watch([output, autoScroll], () => {
  if (autoScroll.value) log.value?.scrollTo({ position: 'bottom', silent: true });
}, { flush: 'post' });

function updateRows() {
  rows.value = Math.max(8, Math.floor((window.innerHeight - 250) / 19));
}
onMounted(() => {
  updateRows();
  window.addEventListener('resize', updateRows);
  void Promise.allSettled(([
    ['get_cpl_version', version], ['get_system_info', systemInfo], ['get_build_info', buildInfo],
  ] as const).map(([command, target]) => invoke<string>(command).then(value => { target.value = value; })));
});
onUnmounted(() => window.removeEventListener('resize', updateRows));

function saveLogs() {
  const stamp = new Date().toISOString();
  const text = [
    'OpenFrp 跨平台启动器 日志文件',
    `CPL 版本：Beta v${version.value} / ${buildInfo.value}`,
    `生成时间：${stamp}`,
    `平台：${systemInfo.value}`,
    `日志范围：${selected.value === 'all' ? '全部日志' : `隧道 #${selected.value}`}`,
    `日志条目数：${entries.value.length}`, '', output.value,
  ].join('\n');
  try {
    const url = URL.createObjectURL(new Blob([text], { type: 'text/plain;charset=utf-8' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = `openfrp-cpl-logs-${stamp.slice(0, 19).replace(/:/g, '-')}.txt`;
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    message.success('已发起日志文件下载');
  } catch {
    message.error('保存日志失败');
  }
}
function clearLogs() {
  dialog.warning({
    title: '清除日志', content: '确定清除全部运行日志吗？', positiveText: '清除', negativeText: '取消',
    onPositiveClick: () => { globalLogService.clearLogs(); selected.value = 'all'; },
  });
}
</script>

<template>
  <n-space vertical :size="12">
    <n-h2 style="margin-bottom: 1px;">运行日志</n-h2>
    <n-card>
      <n-space vertical>
        <n-space align="center" justify="space-between">
          <n-select v-model:value="selected" :options="logOptions" aria-label="选择隧道日志" style="width: 180px;" />
          <n-space align="center">
            <n-switch v-model:value="autoScroll" aria-label="自动滚动">
              <template #checked>自动滚动开启</template>
              <template #unchecked>自动滚动关闭</template>
            </n-switch>
            <n-button text type="primary" :disabled="!entries.length" @click="saveLogs">保存日志</n-button>
            <n-button text type="primary" :disabled="!logEntries.length" @click="clearLogs">清除日志</n-button>
          </n-space>
        </n-space>
        <div class="log-panel">
          <n-log v-if="entries.length" ref="log" :rows="rows" :log="output" />
          <n-empty v-else description="暂无运行日志" />
        </div>
      </n-space>
    </n-card>
  </n-space>
</template>

<style scoped>
.log-panel { min-height: 90px; }
.log-panel :deep(.n-empty) { padding: 24px; }
</style>
