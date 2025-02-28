<script setup lang="ts">
import { ref } from 'vue';

import { useMessage } from 'naive-ui';

import frpApiGetNodeConf from '@/requests/frpApi/frpApiGetNodeConf';

const message = useMessage();

// const token = ref<string>();
const props = defineProps<{
  proxy: Struct.UserProxy;
  token: string;
  fallback: (value: string) => void;
}>();
const val = ref<string>();

const updateValue = (x: string) => {
  if (x[0] === 're') {
    props.fallback(`./frpc -u ${props.token} -p ${props.proxy.id}`);
  } else if (val.value != null) {
    props.fallback(val.value);
  }
};

const getProxyConfig = () => {
  frpApiGetNodeConf({
    node_id: props.proxy.nid,
  }).then((res) => {
    if (res.data.flag) {
      // 第一项为 FRPC 配置，
      // 其他项目为对应隧道配置
      const afd = (res.data as string).split('\n\n[[proxies]]\n');
      val.value = afd[0].toString();
      afd.shift();
      afd.forEach((x) => {
        if (x.startsWith(`name = "${props.proxy.proxyName}"`)) {
          val.value += `\n\n[[proxies]]\n${x}`;
        }
      });
      props.fallback(`./frpc -u ${props.token} -p ${props.proxy.id}`);
    } else {
      message.error(res.data.msg);
    }
  });
};
getProxyConfig();
</script>

<template>
  <div style="padding-top: 1em">
    <n-skeleton v-if="val == null" height="200px"></n-skeleton>
    <n-collapse
      v-else
      display-directive="show"
      default-expanded-names="re"
      accordion
      @update:expanded-names="updateValue"
    >
      <n-collapse-item name="re" title="简易启动">
        <n-alert title="安全性警告" type="warning">
          该页内容请勿随意发送，如Token泄露请前往个人中心重置。
        </n-alert>
        <br />
        <n-code
          word-wrap
          language="shell"
          :code="`$ frpc -u ${props.token} -p ${proxy.id}`"
          readonly
        ></n-code>
      </n-collapse-item>
      <n-collapse-item name="legacy" title="配置文件">
        <n-scrollbar style="max-height: calc(90vh - 250px)">
          <n-alert style="margin-bottom: 1em; width: 100%" closable title="警告" type="error">
            配置文件仅建议高级 (Geek) 用户使用，我们仅建议有技术能力的用户使用配置文件方式启动。<br />因使用配置文件方式启动导致的问题请自行负责。
          </n-alert>
          <!-- <n-alert style="margin-bottom: 1em; width: 100%" closable title="请注意" type="warning">
            我们现已将所提供的配置文件从旧版 ini 格式更换为新版 toml 格式，请您及时更新至最新版 Frpc
            。<br />新版配置文件仅支持 Frpc 0.50+ 。<br />在旧版 Frpc 可使用
            <n-a href="//api.zyghit.cn/toml-to-ini/" target="_blank">toml转ini工具</n-a>
            。<br />这不是长期解决方案，我们也不保证工具转换的配置文件 100% 可用。
          </n-alert> -->

          <n-code language="toml" :code="val" word-wrap />
        </n-scrollbar>
      </n-collapse-item>
    </n-collapse>
  </div>
</template>
