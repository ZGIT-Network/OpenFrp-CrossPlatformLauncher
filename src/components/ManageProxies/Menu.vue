<script setup lang="ts">
import { Component, h, watch } from 'vue';

import { NIcon } from 'naive-ui';

import {
  BuildOutline,
  CloudOfflineOutline,
  CreateOutline,
  DocumentOutline,
  InformationOutline,
  PlayOutline,
  RefreshOutline,
  TrashOutline,
} from '@vicons/ionicons5';

const props = defineProps<{
  proxy: Struct.UserProxy;
  fallback: (key: string) => void;
  btnStyle: string;
}>();

const renderIcon = (icon: Component, size = '16px') => {
  return () => {
    return h(
      NIcon,
      { size },
      {
        default: () => h(icon),
      },
    );
  };
};

const options = [
  {
    label: '刷新状态',
    key: 'refreshState',
    icon: renderIcon(RefreshOutline),
  },
  {
    label: '启用/禁用隧道',
    key: 'stateChange',
    icon: renderIcon(PlayOutline),
  },
  {
    key: 'menu-divider',
    type: 'divider',
  },
  {
    label: '编辑',
    key: 'editConf',
    icon: renderIcon(CreateOutline),
    disabled: props.proxy.online,
  },
  {
    label: '详情',
    key: 'getInfo',
    icon: renderIcon(InformationOutline),
  },
  {
    key: 'menu-divider',
    type: 'divider',
  },
  {
    label: '删除',
    key: 'deleteProxy',
    icon: renderIcon(TrashOutline),
  },
];

const update = () => {
  options[3].disabled = props.proxy.online;
  options[4].disabled = props.proxy.online;
};

watch(() => props.proxy.online, (newVal) => {
  options[3].disabled = newVal;
  options[4].disabled = newVal;
}, { immediate: true });
</script>
<template>
  <n-dropdown trigger="click" :options="options" @update:show="() => update()" @select="fallback">
    <n-tooltip trigger="hover">
      <template #trigger>
        <n-button :style="btnStyle" circle text>
          <n-icon size="16" :component="BuildOutline" />
        </n-button>
      </template>
      操作
    </n-tooltip>
  </n-dropdown>
</template>
