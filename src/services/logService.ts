import { shallowRef } from 'vue';
import { listen } from '@tauri-apps/api/event';

interface LogEntry {
  text: string;
  tunnel?: string;
}

const MAX_LOGS = 2000;
const MAX_STORED_LOGS = 1000;
const MAX_STORED_CHARS = 400_000;
const STORAGE_KEY = 'frpcLogs';

function loadLogs(): LogEntry[] {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (!stored) return [];
  try {
    const entries: unknown = JSON.parse(stored);
    if (Array.isArray(entries)) {
      return entries.slice(-MAX_STORED_LOGS)
        .filter((entry): entry is LogEntry => typeof entry?.text === 'string');
    }
  } catch {
    // 旧版保存的是带 ANSI 样式标记的文本。
  }
  const template = document.createElement('template');
  return stored.split('\n').slice(-MAX_STORED_LOGS).filter(Boolean).map(line => {
    template.innerHTML = line;
    const text = template.content.textContent || '';
    return { text, tunnel: text.match(/\[隧道 (\d+|link-[\w-]+)\]/)?.[1] };
  });
}

class LogService {
  readonly entries = shallowRef<LogEntry[]>(loadLogs());
  private queued: LogEntry[] = [];
  private timer: ReturnType<typeof setTimeout> | undefined;

  private append(entry: LogEntry) {
    if (this.queued.length === MAX_LOGS) this.queued.shift();
    this.queued.push(entry);
    this.timer ??= setTimeout(() => this.flush(), 100);
  }

  flush() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    if (!this.queued.length) return;
    this.entries.value = [...this.entries.value, ...this.queued].slice(-MAX_LOGS);
    this.queued = [];
    try {
      const recent = this.entries.value.slice(-MAX_STORED_LOGS);
      let start = recent.length;
      let length = 0;
      while (start > 0 && length < MAX_STORED_CHARS) {
        length += recent[--start].text.length;
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(recent.slice(start)));
    } catch {
      // 存储空间不足时仍可查看本次运行期间的日志。
    }
  }

  appendSystemLog(message: string) {
    this.append({ text: `[${new Date().toLocaleTimeString()}] [系统] ${message}` });
  }

  appendTunnelLog(tunnel: string, message: string) {
    const text = message.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '').slice(0, 8192);
    this.append({ text: `[${new Date().toLocaleTimeString()}] [隧道 ${tunnel}] ${text}`, tunnel });
  }

  async attach() {
    const listeners = await Promise.allSettled([
      listen<{ message: string }>('log', ({ payload }) => this.appendSystemLog(payload.message)),
      listen<{ tunnelId: string; message: string }>('frpc-instance-log', ({ payload }) =>
        this.appendTunnelLog(payload.tunnelId, payload.message)),
    ]);
    const close = () => {
      listeners.forEach(result => { if (result.status === 'fulfilled') result.value(); });
      this.flush();
    };
    const failure = listeners.find(result => result.status === 'rejected');
    if (failure) this.appendSystemLog('部分日志监听器初始化失败');
    return close;
  }

  clearLogs() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    this.queued = [];
    this.entries.value = [];
    localStorage.removeItem(STORAGE_KEY);
  }
}


export const globalLogService = new LogService();
