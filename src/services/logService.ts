import { ref, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import ansiToHtml from 'ansi-to-html'
import { useLinkTunnelsStore } from '../stores/linkTunnels'

// 扩展Window接口，添加全局属性
declare global {
  interface Window {
    __logSystemInitialized?: boolean;
    __globalLogService?: LogService;
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

export class LogService {
  private logStore = ref<Map<string, string>>(new Map())
  private tunnelLogIndices = ref<Record<string, Set<string>>>({ 'all': new Set() })
  private cleanupFunctions: (() => void)[] = []
  private activeListeners = ref(new Map<string, boolean>())
  private initialized = false
  private MAX_LOGS_PER_TUNNEL = 500
  private MAX_LOG_COUNT = 10000

  constructor() {
    // 确保只有一个实例
    if (window.__globalLogService) {
      return window.__globalLogService
    }
    window.__globalLogService = this
  }

  // 计算日志内容的散列值
  private hashCode(str: string): string {
    let hash = 0;
    if (str.length === 0) return hash.toString();
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return hash.toString();
  }

  // 解析日志文本内容
  private parseLogText(content: string): { timestamp: string, category: string, tunnelId?: string, message: string } {
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
  private normalizeLogContent(content: string): string {
    return content
      .replace(/<span[^>]*>/g, '')
      .replace(/<\/span>/g, '')
      .trim();
  }

  // 添加日志
  public addLog(content: string): void {
    console.log("添加日志:", content);
    
    const normalizedContent = this.normalizeLogContent(content);
    const hash = this.hashCode(normalizedContent);
    
    // 放宽查重条件 - 对于隧道日志，允许更多日志通过
    const isTunnelLog = content.includes('隧道') && (
      content.includes('client/service.go') || 
      content.includes('启动于配置') || 
      content.includes('连接到节点') || 
      content.includes('start')
    );
    
    // 检查日志是否已存在
    if (!isTunnelLog && this.logStore.value.has(hash)) {
      console.log('避免添加重复日志:', content);
      return;
    }
    
    // 存储日志
    this.logStore.value.set(hash, content);
    this.tunnelLogIndices.value['all'].add(hash);
    
    // 解析日志，更新隧道索引
    const parsed = this.parseLogText(content);
    if (parsed.tunnelId) {
      if (!this.tunnelLogIndices.value[parsed.tunnelId]) {
        this.tunnelLogIndices.value[parsed.tunnelId] = new Set();
      }
      this.tunnelLogIndices.value[parsed.tunnelId].add(hash);
    }
    
    // 保存到localStorage
    this.saveLogsToStorage();
  }

  // 保存日志到localStorage
  private saveLogsToStorage() {
    try {
      const maxLogsToStore = 1000;
      const allLogs = Array.from(this.logStore.value.values());
      const logsToStore = allLogs.length > maxLogsToStore 
        ? allLogs.slice(allLogs.length - maxLogsToStore) 
        : allLogs;
      
      localStorage.setItem('frpcLogs', logsToStore.join('\n'));
      console.log(`保存了 ${logsToStore.length} 条日志到localStorage`);
    } catch (e) {
      console.error('保存日志到localStorage时出错:', e);
      try {
        const allLogs = Array.from(this.logStore.value.values());
        const halfSize = Math.floor(allLogs.length / 2);
        const reducedLogs = allLogs.slice(halfSize);
        localStorage.setItem('frpcLogs', reducedLogs.join('\n'));
        console.warn(`由于存储限制，只保存了 ${reducedLogs.length} 条日志`);
      } catch (e2) {
        console.error('即使减少日志量仍无法保存:', e2);
      }
    }
  }

  // 添加系统日志
  public appendSystemLog(message: string) {
    const timestamp = new Date().toLocaleTimeString();
    const log = `[${timestamp}] [系统] ${message}`;
    console.log(`系统日志: ${message}`);
    this.addLog(log);
  }

  // 添加隧道日志
  public appendTunnelLog(tunnelId: string, message: string) {
    const timestamp = new Date().toLocaleTimeString();
    const coloredMessage = convert.toHtml(message.replace(/\[0m/g, '</span>'));
    
    console.log(`隧道日志 ${tunnelId}: ${message.substring(0, 100)}${message.length > 100 ? '...' : ''}`);
    
    // 用正则统一判断"启动成功"日志
    const successPattern = /start.*success|启动.*成功|隧道.*启动成功|success for proxy/i;
    if (successPattern.test(message)) {
      try {
        window.dispatchEvent(new CustomEvent(`tunnel-${tunnelId}-success`, {
          detail: { message: message }
        }));
        console.log(`已派发 tunnel-${tunnelId}-success 事件`);
      } catch (e) {
        console.error('派发 tunnel-success 事件失败:', e);
      }
    }
    
    this.addLog(`[${timestamp}] [<span style="color: #2080f0">隧道 ${tunnelId}</span>] ${coloredMessage}`);
  }

  // 设置隧道监听器
  public async setupTunnelListener(tunnelId: string): Promise<boolean> {
    if (!tunnelId || typeof tunnelId !== 'string') {
      console.error('无效的隧道ID:', tunnelId);
      return false;
    }
    const listenerKey = `frpc-log-${tunnelId}`;
    
    // 检查监听器是否已经设置
    if (this.activeListeners.value.has(listenerKey)) {
      console.log(`隧道 ${tunnelId} 的监听器已经存在，跳过设置`);
      return true;
    }
    
    try {
      const instanceLogUnlisten = await listen(listenerKey, (event: any) => {
        try {
          console.log(`收到隧道 ${tunnelId} 的日志事件:`, event);
          if (!event || !event.payload || typeof event.payload.message === 'undefined') {
            console.warn(`无效的隧道 ${tunnelId} 日志事件:`, event);
            return;
          }
          this.appendTunnelLog(tunnelId, event.payload.message);
        } catch (logError) {
          console.error(`处理隧道 ${tunnelId} 日志时出错:`, logError);
        }
      });
      
      this.activeListeners.value.set(listenerKey, true);
      this.cleanupFunctions.push(instanceLogUnlisten);
      this.appendSystemLog(`<span style='color: #18A058'>隧道 ${tunnelId} 日志监听器已设置</span>`);
      
      if (!this.tunnelLogIndices.value[tunnelId]) {
        this.tunnelLogIndices.value[tunnelId] = new Set();
      }
      
      window.dispatchEvent(new CustomEvent('tunnel-listener-ready', { detail: { tunnelId } }))
      return true;
    } catch (e) {
      console.error(`设置隧道 ${tunnelId} 监听器过程中发生错误:`, e);
      this.appendSystemLog(`<span style="color: #d03050">设置隧道 ${tunnelId} 日志监听器失败: ${e}</span>`);
      return false;
    }
  }

  // 获取日志数据（供组件使用）
  public getLogStore() {
    return this.logStore
  }

  public getTunnelLogIndices() {
    return this.tunnelLogIndices
  }

  // 读取日志并重建索引
  public loadLogsAndRebuildIndices() {
    // 清空现有数据
    this.logStore.value.clear();
    this.tunnelLogIndices.value = { 'all': new Set() };
    
    // 从localStorage读取日志
    const savedLogs = localStorage.getItem('frpcLogs');
    if (!savedLogs) return;
    
    // 按行处理日志
    const logLines = savedLogs.split('\n');
    for (const line of logLines) {
      if (!line.trim()) continue;
      
      // 计算日志散列值
      const normalizedContent = this.normalizeLogContent(line);
      const hash = this.hashCode(normalizedContent);
      
      // 存储日志，避免重复
      if (!this.logStore.value.has(hash)) {
        this.logStore.value.set(hash, line);
        this.tunnelLogIndices.value['all'].add(hash);
        
        // 解析日志，更新隧道索引
        const parsed = this.parseLogText(line);
        if (parsed.tunnelId) {
          if (!this.tunnelLogIndices.value[parsed.tunnelId]) {
            this.tunnelLogIndices.value[parsed.tunnelId] = new Set();
          }
          this.tunnelLogIndices.value[parsed.tunnelId].add(hash);
        }
      }
    }
  }

  // 清除日志
  public clearLogs() {
    this.logStore.value.clear();
    this.tunnelLogIndices.value = { 'all': new Set() };
    localStorage.removeItem('frpcLogs');

    // 清除监听器设置记录
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith('tunnel_listener_set_')) {
        localStorage.removeItem(key);
      }
    }
  }

  // 初始化日志系统
  public async initialize(): Promise<boolean> {
    if (this.initialized) {
      console.log('日志系统已经初始化过');
      return true;
    }

    console.log("初始化全局日志系统");

    // 检查日志系统是否已初始化，避免重复输出初始化消息
    if (!window.__logSystemInitialized) {
      // 首次初始化，添加标记
      window.__logSystemInitialized = true;

      // 添加一条测试日志
      this.appendSystemLog("<span style='color: #18A058'>日志系统已启动</span>");
      console.log('首次初始化日志系统，添加启动日志');
    }

    // 加载已保存的日志
    this.loadLogsAndRebuildIndices();

    try {
      // 监听全局日志
      const globalLogUnlisten = await listen('log', (event: any) => {
        console.log("收到全局日志:", event.payload.message);
        this.addLog(`[${new Date().toLocaleTimeString()}] [系统] ${event.payload.message}`);
      });
      this.cleanupFunctions.push(globalLogUnlisten);
      console.log("已设置全局日志监听器");

      // 监听隧道事件
      const tunnelEventUnlisten = await listen('tunnel-event', async (event: any) => {
        console.log("收到隧道事件:", event.payload);
        const { type, tunnelId, tunnelName } = event.payload;

        // 对于启动相关事件，设置监听器，但避免重复设置
        if ((type === 'start' || type === 'prepare') &&
            !this.activeListeners.value.has(`frpc-log-${tunnelId}`)) {
          console.log(`开始为隧道 ${tunnelId} 设置监听器`);
          await this.setupTunnelListener(tunnelId);
        }

        // 处理不同类型的事件，但不为success类型生成系统日志
        switch (type) {
          case 'prepare':
            this.appendSystemLog(`<span style="color: #2080f0">准备启动隧道 #${tunnelId} ${tunnelName || ''}</span>`);
            break;
          case 'start':
            this.appendSystemLog(`<span style="color: #2080f0">开始启动隧道 #${tunnelId} ${tunnelName || ''}</span>`);
            break;
          case 'stop':
            this.appendSystemLog(`<span style="color: #2080f0">停止隧道 #${tunnelId} ${tunnelName || ''}</span>`);
            break;
          case 'success':
            // 不再添加成功系统日志，因为隧道日志中已有成功信息
            console.log(`收到隧道 ${tunnelId} 成功事件，不生成系统日志`);
            break;
          case 'error':
            this.appendSystemLog(`<span style="color: #d03050">隧道 #${tunnelId} ${tunnelName || ''} 发生错误</span>`);
            break;
        }
      });
      this.cleanupFunctions.push(tunnelEventUnlisten);
      console.log("已设置隧道事件监听器");

      // 为已保存的隧道设置监听器
      const savedStates = localStorage.getItem('tunnelStates');
      if (savedStates) {
        const states = JSON.parse(savedStates);
        for (const id of Object.keys(states)) {
          await this.setupTunnelListener(id);
        }
      }

      // 监听外部隧道事件
      const linkTunnelsStore = useLinkTunnelsStore();
      watch(() => linkTunnelsStore.linkLaunchedTunnels, async (tunnels) => {
        console.log('检测到外部隧道更新:', tunnels);
        if (!tunnels || !tunnels.size) return;

        this.appendSystemLog(`<span style="color: #2080f0">检测到 ${tunnels.size} 个快速启动隧道</span>`);

        // 确保tunnels是可迭代的集合
        for (const tunnelId of tunnels) {
          if (tunnelId && typeof tunnelId === 'string') {
            console.log(`准备设置快速启动隧道监听器: ${tunnelId}`);
            const success = await this.setupTunnelListener(tunnelId);
            if (success) {
              this.appendSystemLog(`<span style="color: #18A058">快速启动隧道 ${tunnelId} 监听器设置成功</span>`);
            }
          }
        }
      }, { immediate: true });

      // 定期检查清理日志
      const pruneInterval = setInterval(() => this.pruneLogsIfNeeded(), 60000);
      this.cleanupFunctions.push(() => clearInterval(pruneInterval));

      // 监听ProxyList发来的setup-tunnel-listener事件
      const setupTunnelListenerHandler = async (e: any) => {
        const tunnelId = e?.detail?.tunnelId
        if (tunnelId) {
          await this.setupTunnelListener(tunnelId)
          window.dispatchEvent(new CustomEvent('tunnel-listener-ready', { detail: { tunnelId } }))
        }
      };
      window.addEventListener('setup-tunnel-listener', setupTunnelListenerHandler);
      this.cleanupFunctions.push(() => {
        window.removeEventListener('setup-tunnel-listener', setupTunnelListenerHandler);
      });

      this.initialized = true;
      console.log("全局日志系统初始化完成");
      return true;

    } catch (error) {
      console.error('设置日志监听器时出错:', error);
      this.appendSystemLog(`设置日志监听器失败: ${error}`);
      return false;
    }
  }

  // 防止日志过长，定期清理
  private pruneLogsIfNeeded() {
    const logCount = this.logStore.value.size;
    if (logCount > this.MAX_LOG_COUNT) {
      console.log(`日志数量(${logCount})超过上限(${this.MAX_LOG_COUNT})，进行清理...`);

      // 为每个隧道保留最新的日志，删除旧日志
      for (const tunnelId of Object.keys(this.tunnelLogIndices.value)) {
        const indexSet = this.tunnelLogIndices.value[tunnelId];
        if (indexSet instanceof Set && indexSet.size > this.MAX_LOGS_PER_TUNNEL) {
          const hashes = Array.from(indexSet);
          const hashesToRemove = hashes.slice(0, indexSet.size - this.MAX_LOGS_PER_TUNNEL);

          console.log(`隧道 ${tunnelId} 日志过多(${indexSet.size})，清理 ${hashesToRemove.length} 条`);

          // 从索引中删除
          for (const hash of hashesToRemove) {
            indexSet.delete(hash);
          }
        }
      }

      // 现在处理全局logStore，只保留所有隧道仍在使用的哈希
      const usedHashes = new Set<string>();
      for (const tunnelId of Object.keys(this.tunnelLogIndices.value)) {
        const indexSet = this.tunnelLogIndices.value[tunnelId];
        if (indexSet instanceof Set) {
          for (const hash of indexSet) {
            usedHashes.add(hash.toString());
          }
        }
      }

      // 删除不再被任何隧道引用的日志
      const hashesToRemove: string[] = [];
      for (const hash of this.logStore.value.keys()) {
        if (!usedHashes.has(hash)) {
          hashesToRemove.push(hash);
        }
      }

      // 执行删除
      for (const hash of hashesToRemove) {
        this.logStore.value.delete(hash);
      }

      console.log(`清理了 ${hashesToRemove.length} 条日志，剩余 ${this.logStore.value.size} 条`);

      // 保存到localStorage
      this.saveLogsToStorage();
    }
  }

  // 清理资源
  public cleanup() {
    this.cleanupFunctions.forEach(cleanup => {
      try {
        cleanup();
      } catch (error) {
        console.error('清理监听器时出错:', error);
      }
    });
    this.cleanupFunctions = [];
    this.activeListeners.value.clear();
    this.initialized = false;
  }
}

// 创建全局日志服务实例
export const globalLogService = new LogService();
