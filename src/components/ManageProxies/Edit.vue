<script lang="ts" setup>
import { Ref, ref, watch, inject } from 'vue';

import { FormItemRule, useMessage } from 'naive-ui';

import { HelpOutline, ShuffleOutline } from '@vicons/ionicons5';

import { invoke } from '@tauri-apps/api/core'
import { NModal, NSelect, NButton } from 'naive-ui';

import './style.less';

const props = defineProps<{
  isEditMode?: boolean;
  editConfig?: Struct.UserProxy;
  nodeConfig?: Ref<Struct.Node>;
  watchDog: Ref<boolean>;
  fallback: (success: boolean, body: Struct.EditOrNewUserProxy | undefined) => void;
}>();

const customConfigString = ref<string>('');

const autoTlsName = ref<string>('false');
// 尝试从父组件注入message，如果不存在则创建本地message
const injectedMessage = inject('message', null);
const message = injectedMessage || useMessage();

const autoTlsType = ref([
  {
    label: '关闭',
    value: '',
  },
  {
    label: '开启',
    value: 'true',
  },
  {
    label: '可输入对应目录下证书文件名',
    value: 'none',
    disabled: true,
  },
]);
const proxyType = ref([
  {
    label: 'TCP',
    value: 'tcp',
    disabled: !props.editConfig && !props.nodeConfig?.value.protocolSupport.tcp,
  },
  {
    label: 'UDP',
    value: 'udp',
    disabled: !props.editConfig && !props.nodeConfig?.value.protocolSupport.udp,
  },
  {
    label: 'HTTP',
    value: 'http',
    disabled: !props.editConfig && !props.nodeConfig?.value.protocolSupport.http,
  },
  {
    label: 'HTTPS',
    value: 'https',
    disabled: !props.editConfig && !props.nodeConfig?.value.protocolSupport.https,
  },
]);

const proxyData = ref<Struct.EditOrNewUserProxy>({
  proxy_id: BigInt(-1),
  dataEncrypt: false,
  dataGzip: false,
  domain_bind: '',
  local_addr: '127.0.0.1',
  local_port: null,
  custom: '',
  name: '',
  node_id: -1,
  remote_port: -1,
  type: 'tcp',
  autoTls: 'false',
  forceHttps: false,
  proxyProtocolVersion: false,
});
const proxyDomainRebind = ref<string>();

const proxyRule = ref({
  node: {
    required: !props.editConfig,
    message: '请选择节点',
    validator() {
      return (
        props.isEditMode ||
        (props.nodeConfig?.value.name !== '' && props.nodeConfig?.value.name !== null)
      );
    },
  },
  name: {
    required: true,
    message: '请输入隧道名称',
    trigger: 'blur',
  },
  local_addr: {
    required: true,
    message: '请输入本地地址',
    trigger: 'blur',
  },
  local_port: {
    required: true,
    validator(rule: FormItemRule, value: number | string | null) {
      if (value === null || value === -1 || value === '-1') return false;
    },
    message: '请输入本地端口',
    trigger: 'blur',
  },
});

function remotePortRule() {
  return {
    required:
      !props.isEditMode && (proxyData.value.type === 'tcp' || proxyData.value.type === 'udp'),
    validator(rule: FormItemRule, value: number | null) {
      if (value === null || value === -1) return false;
    },
    message: '请输入远程端口',
    trigger: 'blur',
  };
}

function domainsRule() {
  return {
    required: proxyData.value.type === 'http' || proxyData.value.type === 'https',
    validator() {
      // console.log('v: ',proxyDomainRebind.value)
      if (!proxyDomainRebind.value || proxyDomainRebind.value === '') return false;
    },
    message: '请输入绑定域名',
    trigger: 'blur',
  };
}

function onSelectTypeUpdate(v: string) {
  proxyData.value.type = v;
  if (
    proxyData.value.type === 'http' &&
    (proxyData.value.local_port === null || proxyData.value.local_port === 443)
  ) {
    proxyData.value.local_port = 80;
  } else if (
    proxyData.value.type === 'https' &&
    (proxyData.value.local_port === null || proxyData.value.local_port === 80)
  ) {
    proxyData.value.local_port = 443;
  }
}

const randomNum = (min: number, max: number) => {
  return Math.floor(Math.random() * (max - 1 - (min + 1))) + (min + 1);
};
const randomProxyName = () => {
  const str = 'Aa1Bb2Cc3Dd4Ff6Gg7Hh8Ii9Jj2Ll3Mm4Nn5Oo6Pp7Qq8Rr9Ss1Tt2Uu3Vv4Ww5Xx6Yy7Zz';
  let output = '';
  for (let index = 0; index < 7; index++) {
    output += str[Math.round(Math.random() * (str.length - 1))];
  }
  proxyData.value.name = output;
};
const limit = [25565, 19132, 8233, 80, 443];

function rd2(al: string[]) {
  const rd1 = randomNum(Number(al[0]), Number(al[1]));
  if (limit.includes(rd1)) {
    return rd2(al);
  } else {
    return rd1;
  }
}

function rd3() {
  const rd1 = randomNum(20000, 65535);
  if (limit.includes(rd1)) {
    return rd3();
  } else {
    return rd1;
  }
}

const randomProxyPort = () => {
  if (props.nodeConfig?.value.allowPort != null && props.nodeConfig?.value.allowPort !== '') {
    const al = props.nodeConfig?.value.allowPort.replace('(', '').replace(')', '').split(',');

    proxyData.value.remote_port = rd2(al);
  } else {
    proxyData.value.remote_port = rd3();
  }
};

const noSideSpaceAndNoRepeatSplit = (value: string) =>
  !value.startsWith(' ') &&
  !value.endsWith(' ') &&
  ((value.endsWith(',') && value[value.length - 2] !== ',') || !value.endsWith(','));

watch(props.watchDog, (x) => {
  if (x) {
    if (proxyData?.value) {
      if (!props.isEditMode && props.nodeConfig) {
        proxyData.value.node_id = props.nodeConfig.value.id;
      }
      if (proxyDomainRebind.value != null) {
        proxyData.value.domain_bind = JSON.stringify(proxyDomainRebind.value.split(','));
      }
      if (proxyData.value.local_addr === 'localhost') {
        proxyData.value.local_addr = '127.0.0.1';
      }
      proxyData.value.autoTls = autoTlsName.value;

      formRef.value.validate((error: any) => {
        if (!error) {
          // do anything

          proxyData.value.custom = customConfigString.value;

          props.fallback(true, proxyData.value);
        } else {
          message.error('请正确的填写数据。');
          props.fallback(false, undefined);
        }
      });
      // 不直接 fallback
      // props.fallback(proxyData.value);
    }
  }
});
if (props.nodeConfig) {
  watch(props.nodeConfig, (x) => {
    proxyData.value.type = 'tcp';
    for (let i = 0; i < proxyType.value.length; i++) {
      const element = proxyType.value[i];
      switch (element.value) {
        case 'tcp': {
          proxyType.value[i].disabled = !props.isEditMode && !x.protocolSupport.tcp;
          break;
        }
        case 'udp': {
          proxyType.value[i].disabled = !props.isEditMode && !x.protocolSupport.udp;
          break;
        }
        case 'http': {
          proxyType.value[i].disabled = !props.isEditMode && !x.protocolSupport.http;
          break;
        }
        case 'https': {
          proxyType.value[i].disabled = !props.isEditMode && !x.protocolSupport.https;
          break;
        }
        default: {
          break;
        }
      }
    }
  });
}

if (props.isEditMode && props.editConfig) {
  // 在这里，先处理一下custom实现
  // const regExpForAutoTls = /auto_tls = [A-Za-z.]{0,65535}(\n)?/;
  // const regExpForForceHttps = /force_https = [A-Za-z]{0,5}(\n)?/;

  // const regExpResultForAutoTls = regExpForAutoTls.exec(props.editConfig.custom);
  // if (regExpResultForAutoTls !== null && regExpResultForAutoTls.length > 0) {
  //   // todo::
  //   const nfff = regExpResultForAutoTls[0].replace('auto_tls = ', '').replace('\n', '');

  //   autoTlsType.value[2].label = nfff;
  //   autoTlsType.value[2].value = nfff;
  //   autoTlsName.value = nfff;
  //   autoTlsType.value[2].disabled = false;
  //   console.log(autoTlsType.value);
  // }

  // const regExpResultForceHttps = regExpForForceHttps.exec(props.editConfig.custom);
  // if (regExpResultForceHttps !== null && regExpResultForceHttps.length > 0) {
  //   const cffResForceHttps = regExpResultForceHttps[0].replace('force_https = ', '');
  //   if (cffResForceHttps === 'true') {
  //     forceHttps.value = true;
  //   }
  // }

  customConfigString.value = props.editConfig.custom;
  proxyData.value = {
    custom: customConfigString.value,
    proxy_id: props.editConfig.id,
    dataGzip: props.editConfig.useCompression,
    dataEncrypt: props.editConfig.useEncryption,
    domain_bind: props.editConfig.domain,
    local_addr: props.editConfig.localIp,
    local_port: props.editConfig.localPort,
    remote_port: props.editConfig.remotePort,
    node_id: props.editConfig.nid,
    name: props.editConfig.proxyName,
    type: props.editConfig.proxyType,
    autoTls: props.editConfig.autoTls,
    proxyProtocolVersion: props.editConfig.proxyProtocolVersion,
    forceHttps: props.editConfig.forceHttps,
  };
  if (proxyData.value.type === 'http' || proxyData.value.type === 'https') {
    proxyDomainRebind.value = (JSON.parse(proxyData.value.domain_bind) as string[]).join(',');
  }
}

const formRef = ref();

const showPortDialog = ref(false);
const portList = ref<{ port: number; type: string; pid?: number; process?: string }[]>([]);
const selectedPortCard = ref<number | null>(null);
const loadingPorts = ref(false);

async function openPortDialog() {
  showPortDialog.value = true;
  loadingPorts.value = true;
  try {
    const res = await invoke('get_local_ports');
    let tcp = (res as any).tcp || [];
    let udp = (res as any).udp || [];
    if (tcp.length && typeof tcp[0] === 'number') tcp = tcp.map((p:number)=>({port:p,type:'TCP'}));
    if (udp.length && typeof udp[0] === 'number') udp = udp.map((p:number)=>({port:p,type:'UDP'}));
    portList.value = [...tcp.map((x:any)=>({...x,type:'TCP'})), ...udp.map((x:any)=>({...x,type:'UDP'}))];
  } catch (e) {
    message.error('获取本地端口失败');
    portList.value = [];
  }
  loadingPorts.value = false;
}

function handlePortCardClick(port: number) {
  selectedPortCard.value = port;
}

function handlePortSelect() {
  if (selectedPortCard.value) {
    proxyData.value.local_port = selectedPortCard.value;
    showPortDialog.value = false;
    message.success(`已选择本地端口: ${selectedPortCard.value}`);
  }
}
</script>

<template>
  <div class="managechannel-edit">
    <n-form ref="formRef" :model="proxyData" :rules="proxyRule">
      <div>
        <n-layout>
          <n-scrollbar
            :class="isEditMode ? '' : 'ovf'"
            :style="isEditMode ? 'max-height: calc(90vh - 130px);' : ''"
          >
            <n-form-item v-if="!isEditMode" path="node" label="已选择节点">
              <n-input
                :value="
                  nodeConfig?.value.id && nodeConfig?.value.id > 0
                    ? '#' + nodeConfig?.value.id + ' ' + nodeConfig?.value.name
                    : null
                "
                readonly
                placeholder="请选择节点"
              ></n-input>
            </n-form-item>
            <!-- :render-label="renderMenuLabel" -->
            <n-grid cols="1 350:2" x-gap="16" y-gap="16">
              <n-gi>
                <n-grid y-gap="16" x-gap="16" cols="1">
                  <n-form-item-gi path="name" label="隧道名称" span="1">
                    <n-input
                      show-count
                      maxlength="22"
                      minlength="3"
                      :value="proxyData?.name"
                      placeholder="隧道名称"
                      @update:value="
                        (x: string) => {
                          proxyData.name = x;
                        }
                      "
                    >
                      <template #suffix>
                        <n-tooltip>
                          <template #trigger>
                            <n-button color="#888" text @click="randomProxyName">
                              <n-icon size="20" :component="ShuffleOutline"></n-icon>
                            </n-button>
                          </template>
                          随机隧道名
                        </n-tooltip>
                      </template>
                    </n-input>
                  </n-form-item-gi>
                  <n-form-item-gi path="local_addr" label="本地地址" span="1">
                    <n-input
                      :value="proxyData.local_addr"
                      :default-value="proxyData.local_addr"
                      placeholder="127.0.0.1"
                      @update:value="
                        (x: string) => {
                          proxyData.local_addr = x;
                        }
                      "
                    >
                      <template #suffix>
                        <n-tooltip style="cursor: default">
                          <template #trigger>
                            <n-icon :component="HelpOutline" />
                          </template>
                          大部分情况请填 127.0.0.1 映射到本机项目
                        </n-tooltip>
                      </template>
                    </n-input>
                  </n-form-item-gi>
                </n-grid>
              </n-gi>
              <n-gi>
                <n-grid y-gap="16" x-gap="16" cols="1">
                  <n-form-item-gi label="隧道类型" span="2">
                    <n-select
                      :disabled="isEditMode"
                      :value="proxyData?.type"
                      :options="proxyType"
                      @update:value="onSelectTypeUpdate"
                    ></n-select>
                  </n-form-item-gi>
                  <n-form-item-gi path="local_port" label="本地端口">
                    <n-input-number
                      :value="proxyData?.local_port"
                      :max="65535"
                      :show-button="false"
                      :min="1"
                      placeholder="12345"
                      @update:value="
                        (vl: number | null) => {
                          if (vl) {
                            proxyData.local_port = vl;
                          }
                        }
                      "
                    >
                      <template #suffix>
                        <n-button size="tiny" @click="openPortDialog" style="margin-left: 4px;">选择本地端口</n-button>
                      </template>
                    </n-input-number>
                  </n-form-item-gi>
                </n-grid>
              </n-gi>
              <n-gi v-if="!isEditMode">
                <n-form-item-gi
                  v-if="proxyData.type == 'http' || proxyData.type == 'https'"
                  :rule="domainsRule()"
                  label="绑定域名"
                >
                  <n-input
                    :allow-input="noSideSpaceAndNoRepeatSplit"
                    placeholder="example.com"
                    :value="proxyDomainRebind"
                    :default-value="
                      proxyData?.domain_bind != undefined && proxyData?.domain_bind !== ''
                        ? (JSON.parse(proxyData.domain_bind) as string[]).join(',')
                        : ''
                    "
                    @update:value="(x: string) => (proxyDomainRebind = x)"
                  >
                    <template #suffix>
                      <n-tooltip style="cursor: default">
                        <template #trigger>
                          <n-icon :component="HelpOutline" />
                        </template>
                        使用半角逗号隔开，例如: example.com,example.cn
                      </n-tooltip>
                    </template>
                  </n-input>
                </n-form-item-gi>
                <n-form-item-gi v-else label="远程端口" :rule="remotePortRule()">
                  <n-input-number
                    :value="proxyData?.remote_port === -1 ? undefined : proxyData?.remote_port"
                    :max="65535"
                    :show-button="false"
                    :min="1"
                    placeholder="远程端口"
                    @update:value="
                      (x: number | null) => {
                        if (x) {
                          proxyData.remote_port = x;
                        }
                      }
                    "
                  >
                    <template #suffix>
                      <n-tooltip>
                        <template #trigger>
                          <n-button color="#888" text @click="randomProxyPort">
                            <n-icon size="20" :component="ShuffleOutline"></n-icon>
                            <span> </span>
                          </n-button>
                        </template>
                        随机远程端口
                      </n-tooltip>
                    </template>
                  </n-input-number>
                </n-form-item-gi>
              </n-gi>
            </n-grid>
            <n-grid
              v-if="isEditMode && (proxyData.type == 'http' || proxyData.type == 'https')"
              y-gap="16"
              x-gap="16"
              cols="1"
            >
              <n-gi>
                <n-form-item-gi :rule="domainsRule()" label="绑定域名">
                  <n-input
                    :allow-input="noSideSpaceAndNoRepeatSplit"
                    placeholder="example.com"
                    :value="proxyDomainRebind"
                    :default-value="
                      proxyData?.domain_bind != undefined && proxyData?.domain_bind !== ''
                        ? (JSON.parse(proxyData.domain_bind) as string[]).join(',')
                        : ''
                    "
                    @update:value="(x: string) => (proxyDomainRebind = x)"
                  >
                    <template #suffix>
                      <n-tooltip style="cursor: default">
                        <template #trigger>
                          <n-icon :component="HelpOutline" />
                        </template>
                        使用半角逗号隔开，例如: example.com,example.cn
                      </n-tooltip>
                    </template>
                  </n-input>
                </n-form-item-gi>
              </n-gi>
            </n-grid>
            <n-collapse>
              <n-collapse-item title="高级配置">
                <n-space :vertical="true">
                  <n-grid cols="1 350:2" x-gap="16" y-gap="16">
                    <n-gi>
                      <n-grid y-gap="16" x-gap="16" cols="1">
                        <n-form-item-gi label="数据加密">
                          <n-switch
                            :value="proxyData?.dataEncrypt"
                            @update:value="
                              (x: boolean) => {
                                if (proxyData != null) proxyData.dataEncrypt = x;
                              }
                            "
                          >
                            <!-- 已有数据的情况下 不需要再显示内容 by:Yue 2023-8-17 -->
                            <!-- <template #checked>
                              启用
                            </template>
                            <template #unchecked>
                              禁用
                            </template> -->
                          </n-switch>
                        </n-form-item-gi>
                      </n-grid>
                    </n-gi>
                    <n-gi>
                      <n-grid y-gap="16" x-gap="16" cols="1">
                        <n-form-item-gi label="数据压缩">
                          <n-switch
                            :value="proxyData?.dataGzip"
                            @update:value="
                              (x: boolean) => {
                                if (proxyData != null) proxyData.dataGzip = x;
                              }
                            "
                          >
                            <!-- 已有数据的情况下 不需要再显示内容 by:Yue 2023-8-17 -->
                          </n-switch>
                        </n-form-item-gi>
                      </n-grid>
                    </n-gi>
                  </n-grid>
                  <n-grid cols="1 350:2" x-gap="16" y-gap="16">
                    <n-form-item-gi v-if="proxyData?.type == 'https'" label="强制 HTTPS">
                      <n-switch
                        :value="proxyData?.forceHttps"
                        @update:value="
                          (x: boolean) => {
                            if (proxyData != null) proxyData.forceHttps = x;
                          }
                        "
                      ></n-switch>
                    </n-form-item-gi>
                    <n-form-item-gi
                      v-if="proxyData?.type == 'tcp' || proxyData?.type == 'https'"
                      label="自动 TLS"
                    >
                      <n-select
                        filterable
                        :options="autoTlsType"
                        :on-search="
                          (x: any) => {
                            if (x.length == 0) {
                              autoTlsType[2].disabled = true;
                              autoTlsType[2].label = '输入证书文件名';
                              autoTlsType[2].value = 'none';
                            } else {
                              switch (x) {
                                case 'false': {
                                  return;
                                }
                                case 'true': {
                                  return;
                                }
                                case '开启': {
                                  return;
                                }
                                case '关闭': {
                                  return;
                                }
                                default: {
                                  autoTlsType[2].label = x;
                                  break;
                                }
                              }
                              autoTlsType[2].disabled = false;
                              autoTlsType[2].value = x;
                            }
                          }
                        "
                        :value="autoTlsName"
                        placeholder=""
                        @update:value="
                          (x: any) => {
                            autoTlsName = x;
                          }
                        "
                      ></n-select>
                    </n-form-item-gi>
                    <n-form-item-gi label="Proxy Protocol V2">
                      <n-switch
                        :value="proxyData?.proxyProtocolVersion"
                        @update:value="
                          (x: boolean) => {
                            if (proxyData != null) proxyData.proxyProtocolVersion = x;
                          }
                        "
                      ></n-switch>
                    </n-form-item-gi>
                  </n-grid>
                  <n-text v-if="proxyData?.type == 'udp'" :depth="3"
                    >* UDP 隧道的 ProxyProtocolV2 仅 OpenFRP 节点支持
                  </n-text>

                  <n-form-item label="自定义配置">
                    <n-input
                      :autosize="{
                        minRows: 6,
                      }"
                      :value="customConfigString"
                      size="medium"
                      type="textarea"
                      placeholder="此处可填写更多自定义配置，一行一个，仅允许出现数字、字母、等号、空格和换行符，错误的配置可能导致隧道无法启动。"
                      @update-value="(x: string) => (customConfigString = x)"
                    />
                  </n-form-item>
                </n-space>
              </n-collapse-item>
            </n-collapse>
          </n-scrollbar>
        </n-layout>
      </div>
    </n-form>
    <n-modal v-model:show="showPortDialog" title="选择本地端口" preset="dialog" style="width: 630px; max-height: 480px;">
      <div v-if="loadingPorts" style="text-align:center;padding:32px 0;">
        <n-spin size="large">正在扫描本地端口...</n-spin>
      </div>
      <n-scrollbar v-else style="max-height: 340px; min-height: 120px;">
        <n-space wrap :size="8">
          <n-card
            v-for="item in portList"
            :key="item.type + '-' + item.port + '-' + (item.pid || '')"
            :class="['port-card', { selected: selectedPortCard === item.port }]"
            size="small"
            style="width: 180px; cursor: pointer;margin-bottom: 3px;"
            @click="handlePortCardClick(item.port)"
          >
            <template #header>
              <span style="font-weight:bold;font-size:1.1em;">{{ item.port }}</span>
            
            </template>
            <template #header-extra>
               <n-tag type="info">{{ item.type }}</n-tag> 
            </template>
            <template #default>
              <n-ellipsis :line-clamp="1" style="font-size:0.95em;">{{ item.process || '-' }}</n-ellipsis>
            </template>
            <template #footer>
              <span style="font-size:0.9em;color:#888;">PID: {{ item.pid || '-' }}</span>
            </template>
          </n-card>
        </n-space>
      </n-scrollbar>
      <template #action>
        <n-button type="primary" @click="handlePortSelect" :disabled="!selectedPortCard">确定</n-button>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
        .port-card.selected {
          border: 2px solid #0F6FB8 !important;
          /* background: #e6f7ff; */
        }
</style>