<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NCard, NSpace, NButton, NInput, NLog, NCollapse, NCollapseItem } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { FrpcConfig } from '../types/frpc'

const props = defineProps<{
  config: FrpcConfig
}>()

const emit = defineEmits<{
  (e: 'update:config', config: FrpcConfig): void
}>()

const logs = ref('')
let unlistenLog: any = null

onMounted(async () => {
  unlistenLog = await listen(`frpc-log-${props.config.id}`, (event: any) => {
    logs.value += event.payload.message + '\n'
  })
})

onUnmounted(() => {
  if (unlistenLog) unlistenLog()
})

const startFrpc = async () => {
  try {
    await invoke('start_frpc_instance', { 
      id: props.config.id,
      token: props.config.token,
      tunnelId: props.config.tunnelId
    })
    emit('update:config', { ...props.config, status: 'running' })
  } catch (e) {
    console.error(e)
  }
}

const stopFrpc = async () => {
  try {
    await invoke('stop_frpc_instance', { id: props.config.id })
    emit('update:config', { ...props.config, status: 'stopped' })
  } catch (e) {
    console.error(e)
  }
}
</script>

<template>
  <n-card :title="`隧道 ${config.tunnelId}`">
    <n-space vertical>
      <n-space>
        <n-input v-model:value="config.token" placeholder="用户Token" />
        <n-input v-model:value="config.tunnelId" placeholder="隧道ID" />
        <n-button 
          @click="startFrpc" 
          :disabled="config.status === 'running'"
        >
          启动
        </n-button>
        <n-button 
          @click="stopFrpc" 
          :disabled="config.status === 'stopped'"
        >
          停止
        </n-button>
      </n-space>
      <n-collapse>
        <n-collapse-item title="运行日志">
          <n-log 
            :rows="10"
            :log="logs"
            :loading="false"
            trim
          />
        </n-collapse-item>
      </n-collapse>
    </n-space>
  </n-card>
</template> 