<script setup lang="ts">
import { defineProps } from 'vue';

const props = defineProps<{
  proxy: Struct.UserProxy;
  copyFallback: (url: string) => void;
}>();
const copyLink = (url: string) => {
  props.copyFallback(url);
};

// 用于解析域名的辅助函数
const parseDomain = (domainStr: string) => {
  try {
    const domains = JSON.parse(domainStr);
    return Array.isArray(domains) ? domains : [];
  } catch (e) {
    return [];
  }
};

// 检查是否为TCP/UDP类型
const isTcpOrUdp = (proxyType: string) => {
  return proxyType === 'tcp' || proxyType === 'udp';
};

// 检查是否显示远程端口
const shouldShowRemotePort = (proxyType: string, remotePort: number) => {
  return (proxyType === 'http' || proxyType === 'https') && remotePort > 0 && remotePort !== 80 && remotePort !== 443;
};
</script>
<template>
  <n-scrollbar style="max-height: calc(100vh - 150px); overflow: auto">
    <n-grid y-gap="8" cols="1">
      <!-- 基本信息 -->
      <n-gi>
        <n-descriptions bordered :cols="2">
          <n-descriptions-item label="ID">
            {{ proxy.id }}
          </n-descriptions-item>
          <n-descriptions-item label="映射对象">
            {{ proxy.proxyType.toUpperCase() }} {{ proxy.localIp }}:{{ proxy.localPort }}
          </n-descriptions-item>
          <n-descriptions-item label="节点">
            #{{ proxy.nid }} {{ proxy.friendlyNode }}
          </n-descriptions-item>
          <!-- 只在HTTP/HTTPS类型显示远程端口 -->
          <n-descriptions-item v-if="!isTcpOrUdp(proxy.proxyType) && proxy.remotePort > 0" label="远程端口">
            {{ proxy.remotePort }}
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      
      <!-- 链接地址 - TCP/UDP类型 -->
      <n-gi v-if="isTcpOrUdp(proxy.proxyType)">
        <n-descriptions bordered :cols="1">
          <n-descriptions-item label="链接地址">
            <n-space vertical size="small">
              <!-- 连接地址 -->
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
              
              <!-- CNAME地址 - 如果存在 -->
              <n-tooltip v-if="proxy.nodeHostname" trigger="hover">
                <template #trigger>
                  <n-button text @click="() => copyLink(proxy.nodeHostname)">
                    <n-ellipsis style="max-width: 100%" :tooltip="false">
                      {{ proxy.nodeHostname }} (CNAME)
                    </n-ellipsis>
                  </n-button>
                </template>
                点击复制CNAME地址
              </n-tooltip>
              
              <!-- 端口信息 -->
              <n-text v-if="proxy.remotePort > 0" depth="3" size="small">
                远程端口: {{ proxy.remotePort }}
              </n-text>
            </n-space>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      
      <!-- 链接地址 - HTTP/HTTPS类型（无域名） -->
      <n-gi v-if="!isTcpOrUdp(proxy.proxyType) && !proxy.domain">
        <n-descriptions bordered :cols="1">
          <n-descriptions-item label="链接地址">
            <n-space vertical size="small">
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
              
              <!-- 显示HTTP/HTTPS协议的端口 -->
              <n-tooltip v-if="shouldShowRemotePort(proxy.proxyType, proxy.remotePort)" trigger="hover">
                <template #trigger>
                  <n-button text @click="() => copyLink(`${proxy.connectAddress}:${proxy.remotePort}`)">
                    <n-ellipsis style="max-width: 100%" :tooltip="false">
                      {{ proxy.connectAddress }}:{{ proxy.remotePort }}
                    </n-ellipsis>
                  </n-button>
                </template>
                点击复制带端口的链接
              </n-tooltip>
            </n-space>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      
      <!-- 绑定域名 - 仅HTTP/HTTPS类型 -->
      <n-gi v-if="!isTcpOrUdp(proxy.proxyType) && proxy.domain">
        <n-descriptions bordered :cols="1">
          <n-descriptions-item label="绑定域名">
            <n-space vertical size="small">
              <template v-for="(domain, index) in parseDomain(proxy.domain)" :key="index">
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button text @click="() => copyLink(domain)">
                      <n-ellipsis style="max-width: 100%" :tooltip="false">
                        {{ domain }}
                      </n-ellipsis>
                    </n-button>
                  </template>
                  点击复制域名
                </n-tooltip>
                <!-- 显示HTTP/HTTPS协议的端口 -->
                <n-tooltip v-if="shouldShowRemotePort(proxy.proxyType, proxy.remotePort)" trigger="hover">
                  <template #trigger>
                    <n-button text @click="() => copyLink(`${domain}:${proxy.remotePort}`)">
                      <n-ellipsis style="max-width: 100%" :tooltip="false">
                        {{ domain }}:{{ proxy.remotePort }}
                      </n-ellipsis>
                    </n-button>
                  </template>
                  点击复制带端口的域名
                </n-tooltip>
              </template>
            </n-space>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      
      <!-- CNAME地址 - 仅HTTP/HTTPS类型和有域名时显示 -->
      <n-gi v-if="!isTcpOrUdp(proxy.proxyType) && proxy.domain && proxy.nodeHostname">
        <n-descriptions bordered :cols="1">
          <n-descriptions-item label="CNAME地址">
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button text @click="() => copyLink(proxy.nodeHostname)">
                  <n-ellipsis style="max-width: 100%" :tooltip="false">
                    {{ proxy.nodeHostname }}
                  </n-ellipsis>
                </n-button>
              </template>
              点击复制CNAME地址
            </n-tooltip>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
      
      <!-- 扩展地址 -->
      <n-gi v-if="proxy.extAddress && proxy.extAddress.length > 0">
        <n-descriptions bordered :cols="1">
          <n-descriptions-item>
            <template #label>
              <n-space align="center">
                <span>扩展地址</span>
                <n-text depth="3" style="font-size: xx-small;">
                  (如果您不知道这是什么意思, 请忽略它)
                </n-text>
              </n-space>
            </template>
            <n-space vertical size="small">
              <template v-for="(eAddr, index) in proxy.extAddress" :key="index">
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button text @click="() => copyLink(eAddr)">
                      <n-ellipsis style="max-width: 100%" :tooltip="false">
                        {{ eAddr }}
                      </n-ellipsis>
                    </n-button>
                  </template>
                  点击复制扩展地址
                </n-tooltip>
              </template>
            </n-space>
          </n-descriptions-item>
        </n-descriptions>
      </n-gi>
    </n-grid>
  </n-scrollbar>
</template>
