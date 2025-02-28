<script setup lang="ts">
import { AlertOutline, CheckmarkOutline, PersonCircleOutline, Warning } from '@vicons/ionicons5';

defineProps<{
  node: Struct.Node & { hostnameExt?: string };
  // selectedId: number | undefined;
  fallback: (node: Struct.Node) => void;
}>();
const codeToState = (code: number) => {
  switch (code) {
    case 401: {
      return { text: '禁用', type: 'error' };
    }
    case 501: {
      return { text: '维护', type: 'warning' };
    }
    case 500: {
      return { text: '离线', type: 'error' };
    }
    case 503: {
      return { text: '异常', type: 'error' };
    }
    case 504: {
      return { text: '超时', type: 'error' };
    }
    default: {
      break;
    }
  }
};
</script>
<template>
  <!-- 'node-selected': selectedId === node.id -->
  <n-card
    :class="{ 'node-selector': true, 'node-disabled': node.fullyLoaded }"
    style="min-height: 140px"
    embedded
    footer-style="padding:12px 16px 12px 16px;"
    content-style="padding:16px 16px 0px 16px;"
    @click="() => fallback(node)"
  >
    <n-space justify="space-between" vertical>
      <n-space vertical :size="[0, 2]">
        <n-space justify="space-between" :size="[8, 0]">
          <span style="font-size: 1rem">#{{ node.id }} {{ node.name }}</span>
          <n-tag
            v-if="
              !node.group.split(';').includes('normal') && !node.group.split(';').includes('vip')
            "
            :bordered="false"
            type="error"
            size="small"
          >
            SVIP
          </n-tag>
          <n-tag
            v-else-if="!node.group.split(';').includes('normal')"
            :bordered="false"
            type="warning"
            size="small"
          >
            VIP
          </n-tag>
        </n-space>
        <n-space :size="[4, 4]">
          <n-tag round :bordered="false" type="success" size="small">
            <template #icon>
              <span style="width: 0">&nbsp;</span>
            </template>
            {{ node.bandwidth === 1024 ? '1Gbps' : `${node.bandwidth}Mbps`
            }}{{
              node.bandwidthMagnification == 1
                ? ''
                : ` *
            ${node.bandwidthMagnification}`
            }}
          </n-tag>
          <n-tag
            v-if="node.needRealname && node.classify !== 1"
            round
            :bordered="false"
            type="warning"
            size="small"
          >
            <template #icon>
              <n-icon :component="PersonCircleOutline" />
            </template>
            实名
          </n-tag>
          <n-tag v-if="node.fullyLoaded" round :bordered="false" type="error" size="small">
            <template #icon>
              <n-icon :component="AlertOutline" />
            </template>
            满载
          </n-tag>
          <n-tag
            v-if="node.status != 200"
            round
            :bordered="false"
            :type="codeToState(node.status)?.type == 'warning' ? 'warning' : 'error'"
            size="small"
          >
            <template #icon>
              <n-icon :component="AlertOutline" />
            </template>
            {{ codeToState(node.status)?.text }}
          </n-tag>
          <!-- 因为所有节点均有TCP无需再次展示 -->

          <n-tag v-if="node.protocolSupport.udp" round :bordered="false" type="info" size="small">
            <template #icon>
              <n-icon :component="CheckmarkOutline" />
            </template>
            UDP
          </n-tag>
          <n-tag v-if="node?.hostnameExt !== ''" round :bordered="false" type="warning" size="small">
            <template #icon>
              <n-icon
                style="padding-bottom: 2px; padding-left: 2px; margin-right: -4px"
                :component="Warning"
              />
            </template>
            扩展地址
          </n-tag>
          <n-tag
            v-if="node.protocolSupport.https && node.protocolSupport.http"
            round
            :bordered="false"
            type="info"
            size="small"
          >
            <template #icon>
              <n-icon :component="CheckmarkOutline" />
            </template>
            HTTP(S)
          </n-tag>
          <n-tag
            v-if="node.protocolSupport.http && !node.protocolSupport.https"
            round
            :bordered="false"
            type="info"
            size="small"
          >
            <template #icon>
              <n-icon :component="CheckmarkOutline" />
            </template>
            HTTP
          </n-tag>
          <n-tag
            v-if="!node.protocolSupport.http && node.protocolSupport.https"
            round
            :bordered="false"
            type="info"
            size="small"
          >
            <template #icon>
              <n-icon :component="CheckmarkOutline" />
            </template>
            HTTPS
          </n-tag>
        </n-space>
        <n-ellipsis :line-clamp="2" style="width: 100%; font-size: 0.9em; color: #888888">
          {{ node.comments }}
        </n-ellipsis>
      </n-space>
    </n-space>
    <!-- <template #footer>
        <n-space vertical size="small">

            <n-space :size="[4, 4]">
                <n-tag :bordered="false" type="success" size="small">
                    {{ node.bandwidth }}Mbps{{ node.bandwidthMagnification == 1 ? '' : ` *
                    ${node.bandwidthMagnification}` }}
                </n-tag>
                <n-tag :bordered="false" v-if="node.needRealname && node.classify !== 1" type="warning" size="small">
                    实名
                </n-tag>
                <n-tag :bordered="false" v-if="node.fullyLoaded" type="error" size="small">
                    满载
                </n-tag>
                <n-tag :bordered="false" v-if="node.status != 200"
                    :type="codeToState(node.status)?.type == 'warning' ? 'warning' : 'error'" size="small">
                    {{ codeToState(node.status)?.text }}
                </n-tag>


                <n-tag :bordered="false" v-if="node.protocolSupport.udp" type="info" size="small">
                    UDP
                </n-tag>
                <n-tag :bordered="false" v-if="node.protocolSupport.https && node.protocolSupport.http" type="info"
                    size="small">
                    HTTP(S)
                </n-tag>
                <n-tag :bordered="false" v-if="node.protocolSupport.http && !node.protocolSupport.https" type="info"
                    size="small">
                    HTTP
                </n-tag>
                <n-tag :bordered="false" v-if="!node.protocolSupport.http && node.protocolSupport.https" type="info"
                    size="small">
                    HTTPS
                </n-tag>
            </n-space>
        </n-space>
    </template> -->
  </n-card>
</template>
