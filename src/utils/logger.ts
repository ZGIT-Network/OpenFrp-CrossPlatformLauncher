// 日志级别
export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
  NONE = 4
}

// 日志项接口
export interface LogEntry {
  timestamp: Date;
  level: LogLevel;
  message: string;
  data?: any;
}

// 日志监听器类型
export type LogListener = (log: LogEntry | null) => void;

// 日志管理类
class Logger {
  private logs: LogEntry[] = [];
  private maxLogs: number = 1000; // 最大日志数量
  private level: LogLevel = LogLevel.INFO; // 默认日志级别
  private listeners: LogListener[] = [];

  constructor() {
    // 从本地存储加载日志级别
    const savedLevel = localStorage.getItem('logLevel');
    if (savedLevel !== null) {
      this.level = parseInt(savedLevel);
    }
  }

  // 设置日志级别
  public setLevel(level: LogLevel): void {
    this.level = level;
    localStorage.setItem('logLevel', level.toString());
  }

  // 获取日志级别
  public getLevel(): LogLevel {
    return this.level;
  }

  // 获取所有日志
  public getLogs(): LogEntry[] {
    return [...this.logs];
  }

  // 添加日志
  private _addLog(level: LogLevel, message: string, data?: any): void {
    if (level < this.level) return;
    
    const log: LogEntry = {
      timestamp: new Date(),
      level,
      message,
      data
    };
    
    this.logs.unshift(log);
    
    // 限制日志数量
    if (this.logs.length > this.maxLogs) {
      this.logs.pop();
    }
    
    // 通知监听器
    this.listeners.forEach(listener => listener(log));
    
    // 控制台输出
    const consoleMethod = level === LogLevel.ERROR ? 'error' : 
                          level === LogLevel.WARN ? 'warn' : 
                          level === LogLevel.INFO ? 'info' : 'debug';
    console[consoleMethod](`[${log.timestamp.toISOString()}] ${message}`, data || '');
  }

  // 日志方法
  public debug(message: string, data?: any): void {
    this._addLog(LogLevel.DEBUG, message, data);
  }

  public info(message: string, data?: any): void {
    this._addLog(LogLevel.INFO, message, data);
  }

  public warn(message: string, data?: any): void {
    this._addLog(LogLevel.WARN, message, data);
  }

  public error(message: string, data?: any): void {
    this._addLog(LogLevel.ERROR, message, data);
  }

  // 添加监听器
  public addListener(listener: LogListener): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter(l => l !== listener);
    };
  }

  // 清空日志
  public clear(): void {
    this.logs = [];
    // 通知监听器
    this.listeners.forEach(listener => listener(null));
  }

  // 导出日志为文本
  public exportLogs(): string {
    return this.logs.map(log => {
      const levelName = LogLevel[log.level];
      const timestamp = log.timestamp.toISOString();
      const dataStr = log.data ? JSON.stringify(log.data) : '';
      return `[${timestamp}] [${levelName}] ${log.message} ${dataStr}`;
    }).join('\n');
  }
}

// 创建单例实例
export const logger = new Logger();

// 导出日志方法
export const debug = logger.debug.bind(logger);
export const info = logger.info.bind(logger);
export const warn = logger.warn.bind(logger);
export const error = logger.error.bind(logger);