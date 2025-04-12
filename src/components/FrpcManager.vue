<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMessage, NButton, NCard, NLog, NSpace, NSwitch, NSelect } from 'naive-ui'
import type { LogInst, SelectOption } from 'naive-ui'
import hljs from 'highlight.js'
import ansiToHtml from 'ansi-to-html'
import { useLinkTunnelsStore } from '../stores/linkTunnels'

// 扩展Window接口，添加全局属性
declare global {
  interface Window {
    __logSystemInitialized?: boolean;
  }
}

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
const MAX_LOGS_PER_TUNNEL = 500; // 每个隧道最大日志条数
const MAX_LOG_COUNT = 10000; // 全局最大日志条数，增加以保存更多日志

// 添加全局标记，避免重复初始化日志
const logSystemInitialized = ref(!!window.__logSystemInitialized);

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
  
  // 识别隧道日志 [隧道 ID]，更全面的正则表达式
  const tunnelMatch = remaining.match(/^\[(?:<span[^>]*>)?隧道\s+(\d+|link-[a-zA-Z0-9\-_]+)(?:<\/span>)?\]/);
  if (tunnelMatch) {
    const tunnelId = tunnelMatch[1];
    return {
      timestamp,
      category: '隧道',
      tunnelId,
      message: remaining.replace(/^\[(?:<span[^>]*>)?隧道\s+(?:\d+|link-[a-zA-Z0-9\-_]+)(?:<\/span>)?\]\s*/, '')
    };
  }
  
  // 未识别的日志格式但可能包含隧道信息
  const idInMessageMatch = remaining.match(/隧道\s+#?(\d+|link-[a-zA-Z0-9\-_]+)/);
  if (idInMessageMatch) {
    return {
      timestamp,
      category: '隧道',
      tunnelId: idInMessageMatch[1],
      message: remaining
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
  // 放宽日志归一化处理，保留更多内容差异，避免误判为重复
  // 不去除时间戳和ID等信息，仅去除一些HTML标签
  return content
    .replace(/<span[^>]*>/g, '')
    .replace(/<\/span>/g, '')
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
  // 直接输出到控制台以便调试
  console.log("添加日志:", content);
  
  const normalizedContent = normalizeLogContent(content);
  const hash = hashCode(normalizedContent);
  
  // 放宽查重条件 - 对于隧道日志，允许更多日志通过
  // 如果是隧道日志，跳过重复检查
  const isTunnelLog = content.includes('隧道') && (
    content.includes('client/service.go') || 
    content.includes('启动于配置') || 
    content.includes('连接到节点') || 
    content.includes('start')
  );
  
  // 检查日志是否已存在
  if (!isTunnelLog && logStore.value.has(hash)) {
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
  }
  
  // 保存到localStorage
  saveLogsToStorage();
  
  // 更新显示的日志
  updateDisplayedLogs();
}

// 保存日志到localStorage
function saveLogsToStorage() {
  try {
    // 为了避免存储过大导致性能问题，只保存最近的日志
    const maxLogsToStore = 1000;
    const allLogs = Array.from(logStore.value.values());
    const logsToStore = allLogs.length > maxLogsToStore 
      ? allLogs.slice(allLogs.length - maxLogsToStore) 
      : allLogs;
    
    localStorage.setItem('frpcLogs', logsToStore.join('\n'));
    console.log(`保存了 ${logsToStore.length} 条日志到localStorage`);
  } catch (e) {
    console.error('保存日志到localStorage时出错:', e);
    // 如果存储失败（可能是因为大小限制），尝试清理一半的日志再保存
    try {
      const allLogs = Array.from(logStore.value.values());
      const halfSize = Math.floor(allLogs.length / 2);
      const reducedLogs = allLogs.slice(halfSize);
      localStorage.setItem('frpcLogs', reducedLogs.join('\n'));
      console.warn(`由于存储限制，只保存了 ${reducedLogs.length} 条日志`);
    } catch (e2) {
      console.error('即使减少日志量仍无法保存:', e2);
    }
  }
}

// 修改现有的日志函数
const appendSystemLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  const log = `[${timestamp}] [系统] ${message}`;
  console.log(`系统日志: ${message}`);
  addLog(log);
};

// 添加普通日志的辅助函数
const appendLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  addLog(`[${timestamp}] [系统] ${message}`);
};

// 修复类型错误
const appendTunnelLog = (tunnelId: string, message: string) => {
  const timestamp = new Date().toLocaleTimeString();
  const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'));
  
  console.log(`隧道日志 ${tunnelId}: ${message.substring(0, 100)}${message.length > 100 ? '...' : ''}`);
  
  // 检查是否是启动成功消息
  if (message.includes('start proxy success') || 
      message.includes('start tunnel success') ||
      message.includes('隧道启动成功') ||
      message.includes('success for proxy')) {
    // 发送一个特殊的成功事件
    window.dispatchEvent(new CustomEvent(`tunnel-${tunnelId}-success`, {
      detail: { message: message }
    }));
    
    // 不再在成功时额外添加系统日志，避免重复
    // appendSystemLog(`<span style="color: #18A058">隧道 ${tunnelId} 启动成功</span>`);
  }
  
  addLog(`[${timestamp}] [<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}`);
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
function pruneLogsIfNeeded() {
  const logCount = logStore.value.size;
  if (logCount > MAX_LOG_COUNT) {
    console.log(`日志数量(${logCount})超过上限(${MAX_LOG_COUNT})，进行清理...`);
    
    // 为每个隧道保留最新的日志，删除旧日志
    for (const tunnelId of Object.keys(tunnelLogIndices.value)) {
      const indexSet = tunnelLogIndices.value[tunnelId];
      if (indexSet instanceof Set && indexSet.size > MAX_LOGS_PER_TUNNEL) {
        const hashes = Array.from(indexSet);
        const hashesToRemove = hashes.slice(0, indexSet.size - MAX_LOGS_PER_TUNNEL);
        
        console.log(`隧道 ${tunnelId} 日志过多(${indexSet.size})，清理 ${hashesToRemove.length} 条`);
        
        // 从索引中删除，但不从logStore中删除（避免影响其他隧道的日志）
        for (const hash of hashesToRemove) {
          indexSet.delete(hash);
        }
      }
    }
    
    // 现在处理全局logStore，只保留所有隧道仍在使用的哈希
    const usedHashes = new Set<string>();
    for (const tunnelId of Object.keys(tunnelLogIndices.value)) {
      const indexSet = tunnelLogIndices.value[tunnelId];
      if (indexSet instanceof Set) {
        for (const hash of indexSet) {
          usedHashes.add(hash.toString());
        }
      }
    }
    
    // 删除不再被任何隧道引用的日志
    const hashesToRemove: string[] = [];
    for (const hash of logStore.value.keys()) {
      if (!usedHashes.has(hash)) {
        hashesToRemove.push(hash);
      }
    }
    
    // 如果仍然超过限制，删除一部分最旧的哈希（保留在usedHashes中的）
    if (usedHashes.size > MAX_LOG_COUNT * 0.9) { // 当使用的哈希超过上限的90%
      const excessCount = usedHashes.size - Math.floor(MAX_LOG_COUNT * 0.7); // 削减到70%
      const allHashes = Array.from(logStore.value.keys());
      const oldestHashes = allHashes.slice(0, excessCount);
      hashesToRemove.push(...oldestHashes);
    }
    
    // 执行删除
    for (const hash of hashesToRemove) {
      logStore.value.delete(hash);
    }
    
    console.log(`清理了 ${hashesToRemove.length} 条日志，剩余 ${logStore.value.size} 条`);
    
    // 保存到localStorage
    saveLogsToStorage();
    
    // 更新显示的日志
    updateDisplayedLogs();
  }
}

onMounted(async () => {
  console.log("FrpcManager组件挂载");
  
  // 检查日志系统是否已初始化，避免重复输出初始化消息
  if (!logSystemInitialized.value) {
    // 首次初始化，添加标记
    window.__logSystemInitialized = true;
    logSystemInitialized.value = true;
    
    // 添加一条测试日志
    appendSystemLog("<span style='color: #18A058'>日志系统已启动</span>");
    console.log('首次初始化日志系统，添加启动日志');
  } else {
    console.log('日志系统已经初始化过，跳过初始化日志');
  }
  
  // 初始加载日志并构建索引
  loadLogsAndRebuildIndices();
  console.log('初始加载日志完成，共有日志条数:', logStore.value.size);
  
  try {
    // 监听全局日志
    const globalLogUnlisten = await listen('log', (event: any) => {
      console.log("收到全局日志:", event.payload.message);
      appendLog(event.payload.message);
    });
    cleanupFunctions.value.push(globalLogUnlisten);
    console.log("已设置全局日志监听器");

    // 监听隧道事件
    const tunnelEventUnlisten = await listen('tunnel-event', async (event: any) => {
      console.log("收到隧道事件:", event.payload);
      const { type, tunnelId, tunnelName } = event.payload;
      
      // 对于启动相关事件，设置监听器，但避免重复设置
      if ((type === 'start' || type === 'prepare') && 
          !activeListeners.value.has(`frpc-log-${tunnelId}`)) {
        console.log(`开始为隧道 ${tunnelId} 设置监听器`);
        await setupTunnelListener(tunnelId);
      }
      
      // 处理不同类型的事件，但不为success类型生成系统日志
      switch (type) {
        case 'prepare':
          appendSystemLog(`<span style="color: #2080f0">准备启动隧道 #${tunnelId} ${tunnelName || ''}</span>`);
          break;
        case 'start':
          appendSystemLog(`<span style="color: #2080f0">开始启动隧道 #${tunnelId} ${tunnelName || ''}</span>`);
          break;
        case 'stop':
          appendSystemLog(`<span style="color: #2080f0">停止隧道 #${tunnelId} ${tunnelName || ''}</span>`);
          break;
        case 'success':
          // 不再添加成功系统日志，因为隧道日志中已有成功信息
          // 避免重复显示
          console.log(`收到隧道 ${tunnelId} 成功事件，不生成系统日志`);
          break;
        case 'error':
          appendSystemLog(`<span style="color: #d03050">隧道 #${tunnelId} ${tunnelName || ''} 发生错误</span>`);
          break;
      }
    });
    cleanupFunctions.value.push(tunnelEventUnlisten);
    console.log("已设置隧道事件监听器");
    
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
    return false;
  }
  
  const listenerKey = `frpc-log-${tunnelId}`;
  console.log(`准备设置隧道 ${tunnelId} 的监听器 (${listenerKey})`);
  
  // 检查监听器是否已经设置
  if (activeListeners.value.has(listenerKey)) {
    console.log(`隧道 ${tunnelId} 的监听器已经存在，跳过设置`);
    return true;
  }
  
  try {
    // 先清理可能存在的旧监听器
    if (activeListeners.value.has(listenerKey)) {
      console.log(`清理隧道 ${tunnelId} 的旧监听器`);
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
    console.log(`开始监听隧道 ${tunnelId} 的日志`);
    
    // 首先添加一条系统日志
    if (tunnelId.startsWith('link-')) {
      // 快速启动隧道特殊处理
      appendSystemLog(`<span style="color: #2080f0">开始监听快速启动隧道 ${tunnelId}</span>`);
    } else {
      // 普通隧道
      appendSystemLog(`<span style="color: #2080f0">开始监听隧道 ${tunnelId}</span>`);
    }
    
    // 使用try-catch包装listen调用
    let instanceLogUnlisten;
    try {
      instanceLogUnlisten = await listen(listenerKey, (event: any) => {
        try {
          console.log(`收到隧道 ${tunnelId} 的日志事件:`, event);
          
          if (!event || !event.payload || typeof event.payload.message === 'undefined') {
            console.warn(`无效的隧道 ${tunnelId} 日志事件:`, event);
            return;
          }
          
          appendTunnelLog(tunnelId, event.payload.message);
        } catch (logError) {
          console.error(`处理隧道 ${tunnelId} 日志时出错:`, logError);
        }
      });
      
      // 验证监听器是否正确设置
      console.log(`隧道 ${tunnelId} 监听器设置成功: `, typeof instanceLogUnlisten === 'function');
      
    } catch (listenError) {
      console.error(`无法监听 ${listenerKey} 事件:`, listenError);
      appendSystemLog(`<span style="color: #d03050">无法设置隧道 ${tunnelId} 日志监听器: ${listenError}</span>`);
      return false;
    }
    
    // 监听器设置成功
    activeListeners.value.set(listenerKey, true);
    cleanupFunctions.value.push(instanceLogUnlisten);
    
    // 添加一个测试日志，确认监听器已设置
    // 如果日志系统是第一次初始化，显示详细消息，否则只在控制台输出
    if (logSystemInitialized.value) {
      console.log(`隧道 ${tunnelId} 日志监听器已设置`);
    } else {
      appendSystemLog(`<span style="color: #18A058">隧道 ${tunnelId} 日志监听器已设置</span>`);
    }
    
    // 确保隧道在日志索引中有记录
    if (!tunnelLogIndices.value[tunnelId]) {
      tunnelLogIndices.value[tunnelId] = new Set();
      // 更新隧道选项
      updateTunnelOptions();
    }
    
    return true;
  } catch (e) {
    console.error(`设置隧道 ${tunnelId} 监听器过程中发生错误:`, e);
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
