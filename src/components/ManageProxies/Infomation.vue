<script setup lang="ts">
const props = defineProps<{
  proxy: Struct.UserProxy;
  copyFallback: (url: string) => void;
}>();
const copyLink = (url: string) => {
  props.copyFallback(url);
};
</script>
<template>
  <n-scrollbar style="max-height: calc(100vh - 150px); overflow: auto">
    <n-grid y-gap="8" cols="1">
      <n-gi>
        <n-descriptions :cols="2">
          <n-descriptions-item label="ID">
            {{ proxy.id }}
          </n-descriptions-item>

          <n-descriptions-item label="映射对象">
            {{ proxy.proxyType.toUpperCase() }} {{ proxy.localIp }}:{{ proxy.localPort }}
          </n-descriptions-item>
          <n-descriptions-item label="节点">
            #{{ proxy.nid }} {{ proxy.friendlyNode }}
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      <n-gi>
        <n-descriptions :cols="1">
          <n-descriptions-item v-if="!proxy.domain" label="链接地址">
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button text @click="() => copyLink(proxy.connectAddress)">
                  <n-ellipsis style="max-width: 100%" :tooltip="false">
                    {{ proxy.connectAddress }}
                  </n-ellipsis>
                </n-button>
              </template>
              点击复制链接
            </n-tooltip>
          </n-descriptions-item>
          <n-descriptions-item v-else label="绑定域名">
            <span>
              {{ (JSON.parse(proxy.domain) as string[] | undefined)?.join(', ') }}
            </span>
          </n-descriptions-item>
          <n-gi v-if="proxy.domain">
            <n-descriptions :cols="1">
              <n-descriptions-item label="CNAME地址">
                <span>
                  {{ proxy.nodeHostname }}
                </span>
              </n-descriptions-item>
            </n-descriptions>
          </n-gi>
        </n-descriptions>
      </n-gi>
      <n-gi v-if="proxy.domain">
        <n-descriptions :cols="1">
          <n-descriptions-item label="CNAME地址">
            <span>
              {{ proxy.nodeHostname }}
            </span>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      <n-gi>
        <n-descriptions :cols="1">
          <n-descriptions-item v-if="proxy.extAddress && proxy.extAddress.length > 0">
            <template #label>
              <span>扩展地址</span>
              <span style="font-size: xx-small; color: #888888">
                (如果您不知道这是什么意思, 请忽略它)</span
              >
            </template>
            <n-grid cols="1">
              <template v-for="eAddr in proxy.extAddress" :key="eAddr">
                <n-gi>
                  <n-tooltip trigger="hover">
                    <template #trigger>
                      <n-button text @click="() => copyLink(eAddr)">
                        <n-ellipsis style="max-width: 100%" :tooltip="false">
                          {{ eAddr }}
                        </n-ellipsis>
                      </n-button>
                    </template>
                    点击复制链接
                  </n-tooltip>
                </n-gi>
              </template>
            </n-grid>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
    </n-grid>
  </n-scrollbar>
</template>
