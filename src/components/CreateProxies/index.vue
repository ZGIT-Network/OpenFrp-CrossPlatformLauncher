<script lang="ts" setup>
import { computed, h, inject, Ref, ref } from 'vue';
import { RouterLink, useRouter } from 'vue-router';

// import classNames from 'classnames';
import { useLoadingBar, useMessage, useNotification } from 'naive-ui';

import frpApiGetNodeList from '@/requests/frpApi/frpApiGetNodeList';
import frpApiNewProxy from '@/requests/frpApi/frpApiNewProxy';
import { RefreshOutline } from '@vicons/ionicons5';

import Edit from '../ManageProxies/Edit.vue';
import Card from './Card.vue';

// import frpApiNewProxy from '@/requests/frpApiNewProxy';
import './style.less';

// const total = ref<number>(0);
const { userInfo } = inject<{ userInfo: Ref<Struct.UserInfo> }>('userInfo') || { userInfo: ref() };
const nodeList = ref<Struct.Node[]>();
const filterProOptions = [
  {
    label: '全部',
    value: 'all',
  },
  {
    type: 'group',
    label: '支持的协议',
    key: 'protocols',
    children: [
      {
        label: 'UDP',
        value: 'udp-support',
      },
      {
        label: 'HTTP(S)',
        value: 'web-support',
      },
    ],
  },
  {
    type: 'group',
    label: '特性',
    key: 'feature',
    children: [
      {
        label: '大水管(大带宽)',
        value: 'bigb',
      },
    ],
  },
];

const router = useRouter();
const notification = useNotification();
const message = useMessage();
const loadingBar = useLoadingBar();

const selectedFilterProMode = ref<string>('all');
const watchDog = ref<boolean>(false);
// const isMinimalMode = inject<boolean>('isMinimalMode') || true;

const fallbackConf = (success: boolean, body: Struct.EditOrNewUserProxy | undefined) => {
  if (success && body !== undefined) {
    createProxy(body);
  } else {
    setTimeout(() => {
      watchDog.value = false;
    }, 1000);
  }
};
const handleGetConf = () => {
  watchDog.value = true;
};

const getNodeList = () => {
  nodeList.value = [];
  loadingBar.start();
  frpApiGetNodeList().then((res) => {
    if (res.flag) {
      //      total.value = res.data.total;
      //    sourceList.value = res.data.list || [];
      nodeList.value = Object.freeze(res.data.list);
      loadingBar.finish();
    } else {
      message.error(res.msg);
      loadingBar.error();
    }
  });
};
const createProxy = (body: Struct.EditOrNewUserProxy) => {
  frpApiNewProxy(body)
    .then((res) => {
      if (res.flag) {
        message.info(`隧道 ${body.name} ${res.msg}`);
        router.push('/proxylist');
      } else {
        message.error(res.msg);
      }
    })
    .catch((res) => {
      message.error(res.statusText);
    });
  watchDog.value = false;
};
getNodeList();

const displayCreateTunnelModel = ref<boolean>(false);
const displayLevelEqual = ref<boolean>(true);
const displayFull = ref<boolean>(true);

// const currentState = ref<number>(1);
// const contentRef = ref();
// const expandItem = ref<string | null>('cn-mainland');
const selectedNode = ref<Struct.Node>({
  allowEc: false,
  fullyLoaded: false,
  bandwidth: -1,
  bandwidthMagnification: -1,
  classify: -1,
  comments: '',
  group: '',
  id: -1,
  maxOnlineMagnification: -1,
  name: '',
  needRealname: false,
  port: -1,
  status: -1,
  unitcostEc: -1,
  description: '',
  protocolSupport: {
    tcp: false,
    udp: false,
    http: false,
    https: false,
  },
  allowPort: null,
});
const handleSelected = (node: Struct.Node) => {
  notification.destroyAll();
  if (node.fullyLoaded) {
    notification.error({
      title: `无法选中 #${node.id} ${node.name}`,
      content: '该节点现处于满载状态，可尝试刷新后再次选择或选用其他节点',
      duration: 2500,
    });
    return;
  }
  if (userInfo.value) {
    if (!node.group.includes(userInfo.value.group)) {
      notification.error({
        title: `无法选中 #${node.id} ${node.name}`,
        content: `该节点需要权限 ${!node.group.replaceAll('svip', '').includes('vip') ? 'SVIP' : node.group.includes('vip') ? 'VIP' : '不属于您的组'} 权限`,
        duration: 1500,
      });
      return;
    }
    if (!userInfo.value.realname && node.needRealname) {
      const lc = notification.error({
        title: `无法选中 #${node.id} ${node.name}`,
        content: () =>
          h(
            'div',
            {},
            {
              default: () => [
                '该节点需要您先完成实名认证。\n',
                h(
                  RouterLink,
                  {
                    to: '/usercenter',
                    onClick: () => {
                      lc.destroy();
                    },
                  },
                  '跳转到 个人中心 进行',
                ),
              ],
            },
          ),
        duration: 2500,
      });
      return;
    }
  }

  selectedNode.value = node;
  displayCreateTunnelModel.value = true;
  // currentState.value = 2;
};
const nodeFilter = (classify: number) => {
  if (!nodeList.value || nodeList.value.length == 0 || !userInfo.value) {
    return [];
  }

  return nodeList.value.filter(
    (x) =>
      x.classify == classify &&
      (displayFull.value || !x.fullyLoaded) &&
      (displayLevelEqual.value || x.group.includes(userInfo.value!.group)) &&
      filterPro2(x),
  );
};
function filterPro2(x: Struct.Node) {
  switch (selectedFilterProMode.value) {
    case 'all': {
      return true;
    }
    case 'bigb': {
      // 大宽带
      return x.bandwidth >= 50;
    }
    case 'udp-support': {
      return x.protocolSupport.udp;
    }
    case 'web-support': {
      return x.protocolSupport.http || x.protocolSupport.https;
    }
  }
}
function nodeCountFilter(classify: number) {
  if (!nodeList.value || !userInfo.value) {
    return -1;
  }
  var x: number = 0;
  for (let i = 0; i < nodeList.value.length; i++) {
    const le = nodeList.value[i];
    if (
      le.classify == classify &&
      (displayFull.value || !le.fullyLoaded) &&
      (displayLevelEqual.value || le.group.includes(userInfo.value!.group)) &&
      filterPro2(le)
    ) {
      x++;
    }
    // else if (le.name === selectedNode.value.name) {
    //   selectedNode.value = {
    //     allowEc: false,
    //     fullyLoaded: false,
    //     bandwidth: -1,
    //     bandwidthMagnification: -1,
    //     classify: -1,
    //     comments: '',
    //     group: '',
    //     id: -1,
    //     maxOnlineMagnification: -1,
    //     name: '',
    //     needRealname: false,
    //     port: -1,
    //     status: -1,
    //     unitcostEc: -1,
    //     description: '',
    //     protocolSupport: {
    //       tcp: false,
    //       udp: false,
    //       http: false,
    //       https: false,
    //     },
    //     allowPort: null,
    //   };
    // }
  }
  return x;
}
</script>

<template>
  <div className="createchannel">
    <n-space :size="[0, 16]" vertical>
      <n-h2>创建隧道</n-h2>
      <n-alert
        v-if="!userInfo?.realname"
        style="margin-top: -24px; margin-bottom: 20px"
        title="无法使用 中国内陆 节点"
        type="error"
      >
        <n-flex :size="[0, 0]" vertical>
          <n-text>请尽快进行实名认证以获得最佳服务体验。未实名将无法使用中国大陆地区节点。</n-text>
          <RouterLink to="/usercenter">前往 个人中心 实名认证</RouterLink>
        </n-flex>
      </n-alert>
      <n-flex align="center" :size="[12, 8]" style="margin-top: -24px">
        <n-button tertiary size="medium" @click="getNodeList">
          <template #icon>
            <n-icon :component="RefreshOutline"> </n-icon>
          </template>
          刷新
        </n-button>
        <n-select
          v-model:value="selectedFilterProMode"
          style="width: 200px"
          :options="filterProOptions"
          @update:value="
            (x: string) => {
              selectedFilterProMode = x;
            }
          "
        ></n-select>
        <n-space :size="[8, 0]">
          <n-checkbox
            size="large"
            :checked="displayFull"
            @update:checked="(x: boolean) => (displayFull = x)"
            >满载</n-checkbox
          >
          <n-checkbox
            size="large"
            :checked="displayLevelEqual"
            @update:checked="(x: boolean) => (displayLevelEqual = x)"
            >无权限</n-checkbox
          >
        </n-space>
      </n-flex>
      <!-- <div>
        {{ selectedFilterProMode }}
        显示满载: {{ displayFull }} , 显示无权限: {{ displayLevelEqual }},MinimalMode: {{ isMinimalMode }}
        {{ userInfo?.group }} Realname: {{ userInfo?.realname }}
      </div> -->
      <n-skeleton v-if="!nodeList || nodeList.length == 0" height="30svh"></n-skeleton>
      <n-collapse v-else accordion default-expanded-names="cn-mainland">
        <n-collapse-item name="cn-mainland">
          <template #header>
            <n-flex :size="[4, 0]" style="font-size: 1rem">
              <n-text>中国大陆 </n-text>
              <n-gradient-text>均需实名</n-gradient-text>
              <n-text :depth="3">{{ nodeCountFilter(1) }} 个可用</n-text>
            </n-flex>
          </template>
          <n-flex :size="[8, 8]">
            <template v-for="node in nodeFilter(1)" :key="node.id">
              <Card :fallback="handleSelected" :selected-id="selectedNode?.id" :node="node"></Card>
            </template>
          </n-flex>
        </n-collapse-item>
        <n-collapse-item name="china-hk&china-tw">
          <template #header>
            <n-flex :size="[4, 0]" style="font-size: 1rem">
              <n-text>中国香港 / 中国台湾 / 中国澳门 </n-text>
              <n-gradient-text type="success">免备建站</n-gradient-text>
              <n-text :depth="3">{{ nodeCountFilter(2) }} 个可用</n-text>
            </n-flex>
          </template>
          <n-flex :size="[8, 8]">
            <template v-for="node in nodeFilter(2)" :key="node.id">
              <Card :fallback="handleSelected" :selected-id="selectedNode?.id" :node="node"></Card>
            </template>
          </n-flex>
        </n-collapse-item>
        <n-collapse-item name="other">
          <template #header>
            <n-flex :size="[4, 0]" style="font-size: 1rem">
              <n-text>海外 </n-text>
              <n-gradient-text type="warning">不建议中国大陆使用</n-gradient-text>
              <n-text :depth="3">{{ nodeCountFilter(3) }} 个可用</n-text>
            </n-flex>
          </template>
          <n-flex :size="[8, 8]">
            <template v-for="node in nodeFilter(3)" :key="node.id">
              <Card :fallback="handleSelected" :selected-id="selectedNode?.id" :node="node"></Card>
            </template>
          </n-flex>
        </n-collapse-item>
      </n-collapse>
    </n-space>
    <n-back-top :listen-to="`download`" :right="40" />
    <n-back-top :listen-to="`html`" :right="40" />
    <n-modal
      v-model:show="displayCreateTunnelModel"
      :on-positive-click="
        () => {
          handleGetConf();
          return false;
        }
      "
      :positiveButtonProps="{ loading: watchDog, size: 'medium', disabled: watchDog }"
      preset="dialog"
      title="创建隧道"
      positive-text="创建"
    >
      <Edit
        v-bind:node-config="computed(() => selectedNode)"
        :fallback="fallbackConf"
        v-bind:watch-dog="computed(() => watchDog)"
        :isEditMode="false"
        :editConfig="undefined"
      ></Edit>
    </n-modal>
  </div>
</template>
