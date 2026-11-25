<script setup lang="ts">
import { computed, h } from 'vue';
import type { Component } from 'vue';
import { NIcon } from 'naive-ui';
import type { DropdownOption } from 'naive-ui';
import {
  BuildOutline, CloudOfflineOutline, CreateOutline, DocumentOutline,
  InformationOutline, PlayOutline, RefreshOutline, TrashOutline,
} from '@vicons/ionicons5';

const props = defineProps<{
  proxy: Struct.UserProxy;
  localRunning: boolean;
  busy: boolean;
  fallback: (key: string) => void;
}>();
const icon = (component: Component) => () => h(NIcon, { size: 16 }, () => h(component));
const options = computed<DropdownOption[]>(() => {
  const online = props.localRunning || props.proxy.online;
  return [
    { label: '刷新状态', key: 'refreshState', icon: icon(RefreshOutline) },
    { label: props.proxy.status ? '禁用隧道' : '启用隧道', key: 'stateChange', icon: icon(PlayOutline), disabled: online },
    { type: 'divider', key: 'details-divider' },
    { label: '编辑', key: 'editConf', icon: icon(CreateOutline), disabled: online },
    { label: '详情', key: 'getInfo', icon: icon(InformationOutline) },
    { label: '获取配置', key: 'getConf', icon: icon(DocumentOutline) },
    { type: 'divider', key: 'actions-divider' },
    { label: '强制下线', key: 'forceOff', icon: icon(CloudOfflineOutline), disabled: !online || props.localRunning },
    { label: '删除', key: 'deleteProxy', icon: icon(TrashOutline), disabled: online },
  ];
});
</script>

<template>
  <n-dropdown trigger="click" :options="options" @select="fallback">
    <n-button text :disabled="busy" :aria-label="'隧道操作 ' + proxy.proxyName">
      <n-icon size="16" :component="BuildOutline" />
    </n-button>
  </n-dropdown>
</template>
