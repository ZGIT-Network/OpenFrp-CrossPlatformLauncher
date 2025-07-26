<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useMessage, NButton, NCard, NLog, NSpace, NSwitch, NSelect } from 'naive-ui'
import type { LogInst, SelectOption } from 'naive-ui'
import hljs from 'highlight.js'
import { globalLogService } from '../services/logService'
import { invoke } from '@tauri-apps/api/core';


const message = useMessage()
const logInst = ref<LogInst | null>(null) // 引用 n-log 实例
const autoScroll = ref(true) // 添加自动滚动控制
const logs = ref('')

// 使用全局日志服务的数据
const logStore = globalLogService.getLogStore()
const tunnelLogIndices = globalLogService.getTunnelLogIndices()

// 右键菜单相关
const showDropdown = ref(false)
const dropdownX = ref(0)
const dropdownY = ref(0)
const selectedText = ref('')

// 添加隧道筛选功能
const tunnelOptions = ref<SelectOption[]>([
  { label: '全部日志', value: 'all' }
])
const selectedTunnel = ref('all')

// 计算动态行数
const dynamicRows = ref(23)

// 根据窗口高度计算行数
const calculateRows = () => {
  // 获取窗口高度并计算合适的行数
  // 假设每行大约占用20px高度，根据实际情况调整
  const availableHeight = window.innerHeight - 300 // 减去其他组件占用的高度
  const calculatedRows = Math.max(23, Math.floor(availableHeight / 16)) // 最少显示23行
  dynamicRows.value = calculatedRows
}

// 更新隧道选项
function updateTunnelOptions() {
  // 保留已有的'全部日志'选项
  const options: SelectOption[] = [{ label: '全部日志', value: 'all' }];
  
  // 添加每个隧道的选项
  for (const tunnelId of Object.keys(tunnelLogIndices.value)) {
    if (tunnelId !== 'all') {
      // 区分普通隧道和快速启动隧道
      if (tunnelId.startsWith('link-')) {
        options.push({
          label: `快速隧道 ${tunnelId}`,
          value: tunnelId
        });
      } else {
        options.push({
          label: `隧道 ${tunnelId}`,
          value: tunnelId
        });
      }
    }
  }
  
  // 更新选项
  tunnelOptions.value = options;
}

const currentVersion = ref('v0.1')

const getCurrentVersion = async () => {
    try {
        const version = await invoke('get_cpl_version')
        currentVersion.value = version as string
    } catch (e) {
        currentVersion.value = '获取失败'
        console.error('获取版本失败:', e)
    }
}
getCurrentVersion()

const systemInfo = ref('')
const buildInfo = ref('')

onMounted(async () => {
    try {
        currentVersion.value = await invoke('get_cpl_version')
        systemInfo.value = await invoke('get_system_info')
        buildInfo.value = await invoke('get_build_info')
    } catch (e) {
        console.error('获取版本信息失败:', e)
    }
})

// 保存日志功能 - 使用原生Web API实现
const saveLogs = () => {
  try {
    // 创建一个函数来过滤HTML标签
    const stripHtmlTags = (text: string): string => {
      const div = document.createElement('div');
      div.innerHTML = text;
      return div.textContent || div.innerText || '';
    };

    // 构建包含基本信息的日志内容
    const timestamp = new Date().toISOString()
    let logHeader = `OpenFrp 跨平台启动器 日志文件\n`
    logHeader += `CPL版本: Beta v${currentVersion.value} 构建号：${buildInfo.value}\n`
    logHeader += `生成时间: ${timestamp}\n`
    logHeader += `平台: ${systemInfo.value}\n`
    logHeader += `用户代理: ${navigator.userAgent}\n`
    
    // 添加隧道信息
    if (selectedTunnel.value === 'all') {
      logHeader += `日志范围: 全部隧道\n`
    } else if (selectedTunnel.value.startsWith('link-')) {
      logHeader += `日志范围: 快速隧道 #${selectedTunnel.value}\n`
    } else {
      logHeader += `日志范围: 隧道 #${selectedTunnel.value}\n`
    }
    
    logHeader += `日志条目数: ${logs.value.split('\n').filter(line => line.trim() !== '').length}\n`
    logHeader += `=`.repeat(80) + `\n\n`
    
    // 过滤日志内容中的HTML标签
    const cleanLogs = stripHtmlTags(logs.value);
    
    // 合并头部信息和日志内容
    const fullLogContent = logHeader + cleanLogs
    
    // 创建一个Blob对象包含日志内容
    const blob = new Blob([fullLogContent], { type: 'text/plain;charset=utf-8' })
    
    // 创建一个虚拟的下载链接
    const link = document.createElement('a')
    
    // 设置下载文件名，包含时间戳
    const filenameTimestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
    let filename = `openfrp-cpl-logs-${filenameTimestamp}.txt`
    
    // 如果当前筛选的是特定隧道，则在文件名中包含隧道ID
    if (selectedTunnel.value !== 'all') {
      filename = `openfrp-cpl-logs-tunnel-#${selectedTunnel.value}-${filenameTimestamp}.txt`
    }
    
    // 创建对象URL
    link.href = URL.createObjectURL(blob)
    link.download = filename
    
    // 触发下载
    document.body.appendChild(link)
    link.click()
    
    // 清理 - 移除链接和对象URL
    document.body.removeChild(link)
    URL.revokeObjectURL(link.href)
    
    message.success('日志已成功保存到默认下载位置')
  } catch (error) {
    console.error('保存日志失败:', error)
    message.error('保存日志失败: ' + (error as Error).message)
  }
}


// 右键菜单选项
const dropdownOptions = ref([
  {
    label: '复制',
    key: 'copy'
  },
  {
    label: '全选',
    key: 'selectAll'
  }
])

// 处理右键菜单事件
const handleContextMenu = (e: MouseEvent) => {
  e.preventDefault()
  showDropdown.value = false
  
  // 获取当前选中的文本
  nextTick().then(() => {
    const selection = window.getSelection()
    if (selection && selection.toString().length > 0) {
      selectedText.value = selection.toString()
    } else {
      selectedText.value = ''
    }
    
    showDropdown.value = true
    dropdownX.value = e.clientX
    dropdownY.value = e.clientY
  })
}
// 获取选中文本
const getSelectedText = () => {
  const text = window.getSelection?.() || document.getSelection?.()
  return text ? text.toString() : ''
}

// 处理菜单选项
const handleSelect = (key: string) => {
  showDropdown.value = false
  
  if (key === 'copy') {
    if (selectedText.value) {
      navigator.clipboard.writeText(selectedText.value).then(() => {
        message.success('已复制到剪贴板')
      }).catch(() => {
        message.error('复制失败')
      })
    } else {
      message.warning('没有选中任何内容')
    }
  } else if (key === 'selectAll') {
    const logElement = document.querySelector('.n-log')
    if (logElement) {
      const selection = window.getSelection()
      const range = document.createRange()
      range.selectNodeContents(logElement)
      selection?.removeAllRanges()
      selection?.addRange(range)
      // 更新选中的文本
      selectedText.value = selection?.toString() || ''
    }
  }
}



// 更新显示的日志
function updateDisplayedLogs() {
  console.log(`更新日志显示，当前选择的隧道: ${selectedTunnel.value}`);
  if (!selectedTunnel.value || !tunnelLogIndices.value[selectedTunnel.value]) {
    logs.value = '';
    console.log("没有找到选择的隧道的日志");
    return;
  }
  
  // 获取当前选择的隧道日志哈希列表
  const hashes = Array.from(tunnelLogIndices.value[selectedTunnel.value]);
  
  // 获取对应的日志内容并合并
  const logEntries = hashes.map(hash => logStore.value.get(hash)).filter(Boolean);
  logs.value = logEntries.join('\n');
  
  // 自动滚动
  if (autoScroll.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom', silent: true });
    });
  }
  
  console.log(`更新了 ${logEntries.length} 条日志，总长度: ${logs.value.length} 字符`);
}

// 监听选择的隧道变化
watch(() => selectedTunnel.value, () => {
  updateDisplayedLogs();
}, { immediate: true });

// 监听隧道索引变化，更新隧道选项
watch(() => tunnelLogIndices.value, () => {
  updateTunnelOptions();
  updateDisplayedLogs();
}, { deep: true });

// 添加日志滚动处理函数
const handleScroll = ({ scrollTop, scrollHeight, containerHeight }: any) => {
  // 当用户向上滚动时，禁用自动滚动
  if (scrollHeight - (scrollTop + containerHeight) > 50) {
    autoScroll.value = false;
  } else {
    autoScroll.value = true;
  }
};

// 清除日志
const clearLogs = () => {
  globalLogService.clearLogs();
  logs.value = '';
  
  // 重置筛选器选项
  tunnelOptions.value = [{ label: '全部日志', value: 'all' }];
  selectedTunnel.value = 'all';
};

onMounted(async () => {
  console.log("FrpcManager组件挂载");
  
  // 计算初始行数
  calculateRows()
  
  // 监听窗口大小变化
  window.addEventListener('resize', calculateRows)
  
  // 初始加载日志并构建索引
  globalLogService.loadLogsAndRebuildIndices();
  console.log('初始加载日志完成，共有日志条数:', logStore.value.size);
  
  // 更新隧道选项和显示的日志
  updateTunnelOptions();
  updateDisplayedLogs();

  // 事件通信：监听ProxyList发来的setup-tunnel-listener事件
  window.addEventListener('setup-tunnel-listener', async (e: any) => {
    const tunnelId = e?.detail?.tunnelId
    if (tunnelId) {
      await globalLogService.setupTunnelListener(tunnelId)
      window.dispatchEvent(new CustomEvent('tunnel-listener-ready', { detail: { tunnelId } }))
    }
  })
});

onUnmounted(() => {
  // 移除窗口大小变化监听器
  window.removeEventListener('resize', calculateRows)
  // 组件卸载时不清理全局监听器，只清理组件特定的监听器
  console.log("FrpcManager组件卸载，保持全局日志监听器运行");
});

</script>

<template>
  <n-space vertical>
    <n-h2 style="margin-bottom: 1px;">运行日志 </n-h2> 
    <n-card >
      <template #header>
         <n-select 
            v-model:value="selectedTunnel" 
            :options="tunnelOptions" 
            placeholder="选择隧道" 
            style="width: 180px"
          />
      </template>
      <template #header-extra>
        <n-space>
         
          <n-switch v-model:value="autoScroll">
            <template #checked>自动滚动开启</template>
            <template #unchecked>自动滚动关闭</template>
          </n-switch>
          <n-button text type="primary" @click="saveLogs">
            保存日志
          </n-button>
          <n-button text type="primary" @click="clearLogs">
            清除日志
          </n-button>
        </n-space>
      </template>
      <div @contextmenu="handleContextMenu">
        <n-log 
          :rows="dynamicRows"
          :log="logs"
          :loading="false"
          :hljs="hljs"
          ref="logInst"
          @scroll="handleScroll"
          language="naive-log"
          trim
        />
      </div>
      <n-dropdown
        placement="bottom-start"
        trigger="manual"
        :x="dropdownX"
        :y="dropdownY"
        :options="dropdownOptions"
        :show="showDropdown"
        :on-clickoutside="() => {showDropdown = false}"
        @select="handleSelect"
      />
      <!-- <n-log 
        :rows="dynamicRows"
        :log="logs"
        :loading="false"
        :hljs="hljs"
        ref="logInst"
        @scroll="handleScroll"
        language="naive-log"
        trim
      /> -->
    </n-card>
  </n-space>
</template>