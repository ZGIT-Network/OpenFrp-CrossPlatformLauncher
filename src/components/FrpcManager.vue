<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, NButton, NCard, NLog, NSpace, NSwitch, NSelect } from 'naive-ui'
import type { LogInst, SelectOption } from 'naive-ui'
import hljs from 'highlight.js'
import ansiToHtml from 'ansi-to-html'
import { useLinkTunnelsStore } from '../stores/linkTunnels'


const convert = new ansiToHtml({
  fg: '#FFF',
  bg: '#000',
  newline: true,
  escapeXML: false,
  stream: false,
  colors: {
    '1': '#FB9FB1',  // 红色
    '2': '#ACC267',  // 绿色
    '3': '#DDB26F',  // 黄色
    '4': '#6FC2EF',  // 蓝色
    '5': '#E1A3EE',  // 紫色
    '6': '#12CFC0',  // 青色
    '7': '#D0D0D0',  // 白色
    '9': '#F6363F'   // 亮红色
  }
})

const message = useMessage()
const logInst = ref<LogInst | null>(null) // 引用 n-log 实例
const autoScroll = ref(true) // 添加自动滚动控制
// 使用对象保存日志，key为日志内容散列，防止重复
const logStore = ref<Map<string, string>>(new Map())
const logs = ref('')
const cleanupFunctions = ref<(() => void)[]>([])
const activeListeners = ref(new Map<string, boolean>())

// 添加隧道筛选功能
const tunnelOptions = ref<SelectOption[]>([
  { label: '全部日志', value: 'all' }
])
const selectedTunnel = ref('all')

// 计算日志内容的MD5散列值
function hashCode(str: string): string {
  let hash = 0;
  if (str.length === 0) return hash.toString();
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  return hash.toString();
}

// 隧道日志索引，记录每个隧道的日志索引
const tunnelLogIndices = ref<Record<string, Set<string>>>({
  'all': new Set()
});

// 解析日志文本内容
function parseLogText(content: string): { timestamp: string, category: string, tunnelId?: string, message: string } {
  // 解析形如 [HH:MM:SS] [category] message 的日志
  const timestampMatch = content.match(/^\[(\d{1,2}:\d{1,2}:\d{1,2})\]/);
  const timestamp = timestampMatch ? timestampMatch[1] : '';
  
  let remaining = content.replace(/^\[\d{1,2}:\d{1,2}:\d{1,2}\]\s*/, '');
  
  // 识别系统日志 [系统]
  if (remaining.startsWith('[系统]')) {
    return {
      timestamp,
      category: '系统',
      message: remaining.replace(/^\[系统\]\s*/, '')
    };
  }
  
  // 识别隧道日志 [隧道 ID]
  const tunnelMatch = remaining.match(/^\[(?:<span[^>]*>)?隧道\s+(\d+)(?:<\/span>)?\]/);
  if (tunnelMatch) {
    const tunnelId = tunnelMatch[1];
    return {
      timestamp,
      category: '隧道',
      tunnelId,
      message: remaining.replace(/^\[(?:<span[^>]*>)?隧道\s+\d+(?:<\/span>)?\]\s*/, '')
    };
  }
  
  // 未识别的日志格式
  return {
    timestamp,
    category: '未知',
    message: remaining
  };
}

// 提取日志核心内容，移除变量部分以便去重
function normalizeLogContent(content: string): string {
  return content
    .replace(/\[\d{1,2}:\d{1,2}:\d{1,2}\]/g, '') // 移除时间戳
    .replace(/<span[^>]*>/g, '').replace(/<\/span>/g, '') // 移除HTML标签
    .replace(/\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}\.\d+/g, '') // 移除日期时间
    .replace(/\[[a-f0-9]{16}\]/g, '') // 移除16位十六进制ID
    .trim();
}

// 读取日志并重建索引
function loadLogsAndRebuildIndices() {
  // 清空现有数据
  logStore.value.clear();
  tunnelLogIndices.value = { 'all': new Set() };
  
  // 从localStorage读取日志
  const savedLogs = localStorage.getItem('frpcLogs');
  if (!savedLogs) return;
  
  // 按行处理日志
  const logLines = savedLogs.split('\n');
  for (const line of logLines) {
    if (!line.trim()) continue;
    
    // 计算日志散列值
    const normalizedContent = normalizeLogContent(line);
    const hash = hashCode(normalizedContent);
    
    // 存储日志，避免重复
    if (!logStore.value.has(hash)) {
      logStore.value.set(hash, line);
      tunnelLogIndices.value['all'].add(hash);
      
      // 解析日志，更新隧道索引
      const parsed = parseLogText(line);
      if (parsed.tunnelId) {
        if (!tunnelLogIndices.value[parsed.tunnelId]) {
          tunnelLogIndices.value[parsed.tunnelId] = new Set();
        }
        tunnelLogIndices.value[parsed.tunnelId].add(hash);
      } else if (parsed.category === '系统') {
        // 系统日志只添加到'all'，不再添加到各个隧道
        // 这确保了选择特定隧道时不会显示系统日志
      }
    }
  }
  
  // 更新隧道选项
  updateTunnelOptions();
  
  // 重新生成显示日志
  updateDisplayedLogs();
}

// 更新隧道选项
function updateTunnelOptions() {
  // 保留已有的'全部日志'选项
  const options: SelectOption[] = [{ label: '全部日志', value: 'all' }];
  
  // 添加每个隧道的选项
  for (const tunnelId of Object.keys(tunnelLogIndices.value)) {
    if (tunnelId !== 'all') {
      options.push({
        label: `隧道 ${tunnelId}`,
        value: tunnelId
      });
    }
  }
  
  // 更新选项
  tunnelOptions.value = options;
}

// 更新显示的日志
function updateDisplayedLogs() {
  if (!selectedTunnel.value || !tunnelLogIndices.value[selectedTunnel.value]) {
    logs.value = '';
    return;
  }
  
  // 获取当前选择的隧道日志哈希列表
  const hashes = Array.from(tunnelLogIndices.value[selectedTunnel.value]);
  
  // 按照添加顺序排序 (这里假设添加顺序就是正确的显示顺序)
  // 此处可以根据需要实现其他排序逻辑，例如按时间戳
  
  // 获取对应的日志内容并合并
  const logContent = hashes.map(hash => logStore.value.get(hash)).filter(Boolean).join('\n');
  logs.value = logContent;
  
  // 自动滚动
  if (autoScroll.value) {
    nextTick(() => {
      logInst.value?.scrollTo({ position: 'bottom', silent: true });
    });
  }
}

// 监听选择的隧道变化
watch(() => selectedTunnel.value, () => {
  updateDisplayedLogs();
}, { immediate: true });

// 添加日志滚动处理函数
const handleScroll = ({ scrollTop, scrollHeight, containerHeight }: any) => {
  // 当用户向上滚动时，禁用自动滚动
  if (scrollHeight - (scrollTop + containerHeight) > 50) {
    autoScroll.value = false;
  } else {
    autoScroll.value = true;
  }
};

// 添加日志
function addLog(content: string): void {
  const normalizedContent = normalizeLogContent(content);
  const hash = hashCode(normalizedContent);
  
  // 检查日志是否已存在
  if (logStore.value.has(hash)) {
    console.log('避免添加重复日志:', content);
    return;
  }
  
  // 存储日志
  logStore.value.set(hash, content);
  tunnelLogIndices.value['all'].add(hash);
  
  // 解析日志，更新隧道索引
  const parsed = parseLogText(content);
  if (parsed.tunnelId) {
    if (!tunnelLogIndices.value[parsed.tunnelId]) {
      tunnelLogIndices.value[parsed.tunnelId] = new Set();
      // 更新隧道选项
      updateTunnelOptions();
    }
    tunnelLogIndices.value[parsed.tunnelId].add(hash);
  } else if (parsed.category === '系统') {
    // 只向'all'添加系统日志，不再自动添加到所有隧道
    // 保持系统日志只在"全部日志"选项中显示
  }
  
  // 保存到localStorage
  saveLogsToStorage();
  
  // 更新显示的日志
  updateDisplayedLogs();
}

// 保存日志到localStorage
function saveLogsToStorage() {
  // 取出所有日志内容
  const allLogs = Array.from(logStore.value.values()).join('\n');
  localStorage.setItem('frpcLogs', allLogs);
}

// 修改现有的日志函数
const appendSystemLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  addLog(`[${timestamp}] [系统] ${message}`);
};

// 修复类型错误
const appendTunnelLog = (tunnelId: string, message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'));
  
  // 新增：检查是否是启动成功消息，如果是，发送一个特殊的成功事件
  if (message.includes('start proxy success') || 
      message.includes('start tunnel success') ||
      message.includes('隧道启动成功')) {
    // 发送一个特殊的成功事件，ProxyList组件可以监听这个事件
    window.dispatchEvent(new CustomEvent(`tunnel-${tunnelId}-success`, {
      detail: { message: message }
    }));
  }
  
  addLog(`[${timestamp}] [<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}`);
};

// 添加普通日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  addLog(`[${timestamp}] [系统] ${message}`);
};

// 清除日志
const clearLogs = () => {
  logStore.value.clear();
  tunnelLogIndices.value = { 'all': new Set() };
  localStorage.removeItem('frpcLogs');
  logs.value = '';
  
  // 重置筛选器选项
  // tunnelOptions.value = [{ label: '全部日志', value: 'all' }];
  // selectedTunnel.value = 'all';
  
  // 清除监听器设置记录
  for (const key of Object.keys(localStorage)) {
    if (key.startsWith('tunnel_listener_set_')) {
      localStorage.removeItem(key);
    }
  }
};

// 防止日志过长，定期清理
const MAX_LOG_COUNT = 5000; // 最大日志条数
function pruneLogsIfNeeded() {
  const logCount = logStore.value.size;
  if (logCount > MAX_LOG_COUNT) {
    console.log(`日志数量(${logCount})超过上限(${MAX_LOG_COUNT})，进行清理...`);
    
    // 获取所有日志并按添加顺序排序
    const allHashes = Array.from(logStore.value.keys());
    const hashesToRemove = allHashes.slice(0, logCount - MAX_LOG_COUNT * 0.8); // 保留80%的最新日志
    
    // 删除旧日志
    for (const hash of hashesToRemove) {
      logStore.value.delete(hash);
      
      // 从所有索引中删除
      for (const indexSet of Object.values(tunnelLogIndices.value)) {
        indexSet.delete(hash);
      }
    }
    
    // 保存到localStorage
    saveLogsToStorage();
    
    // 更新显示的日志
    updateDisplayedLogs();
    
    console.log(`已清理${hashesToRemove.length}条日志，剩余${logStore.value.size}条`);
  }
}

onMounted(async () => {
  // 初始加载日志并构建索引
  loadLogsAndRebuildIndices();
  
  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      appendLog(event.payload.message);
    });
    cleanupFunctions.value.push(globalLogUnlisten);

    // 监听隧道事件，修改为每次都重新设置监听器
    const tunnelEventUnlisten = await listen('tunnel-event', async (event: any) => {
      const { type, tunnelId, tunnelName } = event.payload;
      
      // 对于启动相关事件，始终重新设置监听器
      if (type === 'start' || type === 'prepare') {
        await setupTunnelListener(tunnelId);
      }
      
      // 处理不同类型的事件
      switch (type) {
        case 'prepare':
          appendSystemLog(`<span style="color: #2080f0">准备启动隧道 #${tunnelId} ${tunnelName}</span>`);
          break;
        case 'start':
          appendSystemLog(`<span style="color: #2080f0">开始启动隧道 #${tunnelId} ${tunnelName}</span>`);
          break;
        case 'stop':
          appendSystemLog(`<span style="color: #2080f0">停止隧道 #${tunnelId} ${tunnelName}</span>`);
          break;
        case 'success':
          appendSystemLog(`<span style="color: #2080f0">隧道 #${tunnelId} ${tunnelName} 启动成功</span>`);
          break;
        case 'error':
          appendSystemLog(`<span style="color: #2080f0">隧道 #${tunnelId} ${tunnelName} 发生错误</span>`);
          break;
      }
    });
    cleanupFunctions.value.push(tunnelEventUnlisten);

    // 为已保存的隧道设置监听器
    const savedStates = localStorage.getItem('tunnelStates');
    if (savedStates) {
      const states = JSON.parse(savedStates);
      for (const id of Object.keys(states)) {
        await setupTunnelListener(id);
      }
    }

    // 监听外部隧道事件
    const linkTunnelsStore = useLinkTunnelsStore();
    watch(() => linkTunnelsStore.linkLaunchedTunnels, async (tunnels) => {
      console.log('检测到外部隧道更新:', tunnels);
      if (!tunnels || !tunnels.size) return;
      
      appendSystemLog(`<span style="color: #2080f0">检测到 ${tunnels.size} 个快速启动隧道</span>`);
      
      // 确保tunnels是可迭代的集合
      for (const tunnelId of tunnels) {
        if (tunnelId && typeof tunnelId === 'string') {
          console.log(`准备设置快速启动隧道监听器: ${tunnelId}`);
          const success = await setupTunnelListener(tunnelId);
          if (success) {
            appendSystemLog(`<span style="color: #18A058">快速启动隧道 ${tunnelId} 监听器设置成功</span>`);
          }
        }
      }
    }, { immediate: true });

    // 定期检查清理日志
    const pruneInterval = setInterval(pruneLogsIfNeeded, 60000); // 每分钟检查一次

    onUnmounted(() => {
      // 清理所有监听器
      cleanupFunctions.value.forEach(cleanup => {
        try {
          cleanup();
        } catch (error) {
          console.error('清理监听器时出错:', error);
        }
      });
      cleanupFunctions.value = [];
      activeListeners.value.clear();
      
      // 清理定时器
      clearInterval(pruneInterval);
    });

  } catch (error) {
    console.error('设置日志监听器时出错:', error);
    appendSystemLog(`设置日志监听器失败: ${error}`);
    message.error('设置日志监听器失败');
  }
});

// 修改设置隧道监听器的逻辑
const setupTunnelListener = async (tunnelId: string) => {
  // 确保tunnelId是字符串且非空
  if (!tunnelId || typeof tunnelId !== 'string') {
    console.error('无效的隧道ID:', tunnelId);
    return;
  }
  
  const listenerKey = `frpc-log-${tunnelId}`;
  
  // 先清理可能存在的旧监听器
  if (activeListeners.value.has(listenerKey)) {
    console.log(`重新设置隧道 ${tunnelId} 的日志监听器`);
    // 找到并清理旧的监听器
    const index = cleanupFunctions.value.findIndex(
      fn => fn.toString().includes(listenerKey)
    );
    if (index !== -1) {
      try {
        await cleanupFunctions.value[index]();
        cleanupFunctions.value.splice(index, 1);
      } catch (e) {
        console.error(`清理隧道 ${tunnelId} 旧监听器失败:`, e);
      }
    }
    activeListeners.value.delete(listenerKey);
  }
  
  // 设置新的监听器
  console.log(`设置隧道 ${tunnelId} 的日志监听器 (${listenerKey})`);
  try {
    // 首先添加一条系统日志，表明正在监听该隧道
    if (tunnelId.startsWith('link-')) {
      // 快速启动隧道特殊处理
      appendSystemLog(`<span style="color: #2080f0">开始监听快速启动隧道 ${tunnelId}</span>`);
    }
    
    const instanceLogUnlisten = await listen(listenerKey, (event: any) => {
      if (!event || !event.payload || !event.payload.message) {
        console.warn(`收到隧道 ${tunnelId} 的无效日志事件:`, event);
        return;
      }
      console.log(`收到隧道 ${tunnelId} 的日志:`, event.payload.message);
      appendTunnelLog(tunnelId, event.payload.message);
    });
    activeListeners.value.set(listenerKey, true);
    cleanupFunctions.value.push(instanceLogUnlisten);
    
    // 添加一个测试日志，确认监听器已设置
    appendSystemLog(`<span style="color: #18A058">隧道 ${tunnelId} 日志监听器已设置</span>`);
    
    // 如果是快速启动的隧道，确保正确地添加到tunnelLogIndices
    if (!tunnelLogIndices.value[tunnelId]) {
      tunnelLogIndices.value[tunnelId] = new Set();
      // 更新隧道选项
      updateTunnelOptions();
    }
    
    return true;
  } catch (e) {
    console.error(`设置隧道 ${tunnelId} 监听器失败:`, e);
    appendSystemLog(`<span style="color: #d03050">设置隧道 ${tunnelId} 日志监听器失败: ${e}</span>`);
    return false;
  }
}

</script>

<template>
  <n-space vertical>
    <n-card title="运行日志">
      <template #header-extra>
        <n-space>
          <n-select 
            v-model:value="selectedTunnel" 
            :options="tunnelOptions" 
            placeholder="选择隧道" 
            style="width: 180px"
          />
          <n-switch v-model:value="autoScroll">
            <template #checked>自动滚动开启</template>
            <template #unchecked>自动滚动关闭</template>
          </n-switch>
          <n-button text type="primary" @click="clearLogs">
            清除日志
          </n-button>
        </n-space>
      </template>
      <n-log 
        :rows="25"
        :log="logs"
        :loading="false"
        :hljs="hljs"
        ref="logInst"
        @scroll="handleScroll"
        language="naive-log"
        trim
      />
    </n-card>
  </n-space>
</template>
