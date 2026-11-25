import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { reactive, ref, shallowRef } from 'vue';
import { v4 as uuid } from 'uuid';
import { globalLogService } from '@/services/logService';

interface RunningTunnel {
  id: string;
  external: boolean;
}

interface TunnelStart extends RunningTunnel {
  name: string;
  token: string;
  remote: string;
}

export interface TunnelInstance {
  tunnelId: string;
  instanceId: string;
}

export const runningTunnels = shallowRef(new Map<string, RunningTunnel>());
export const frpcDownloading = ref(false);
export const pendingTunnels = reactive(new Set<string>());
const tunnelInstances = new Map<string, string>();
let refreshSequence = 0;

export async function refreshRunningTunnels() {
  const sequence = ++refreshSequence;
  const tunnels = await invoke<RunningTunnel[]>('list_frpc_instances');
  if (sequence === refreshSequence) {
    runningTunnels.value = new Map(tunnels.map(tunnel => [tunnel.id, tunnel]));
  }
}

function readStates(): Record<string, string> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem('tunnelStates') || '{}');
    if (value && typeof value === 'object') {
      return Object.fromEntries(Object.entries(value).filter(([id, status]) => /^\d+$/.test(id) && status === 'running'));
    }
  } catch { /* 兼容损坏的旧设置 */ }
  return {};
}

export function getSavedTunnelIds(owner: number): string[] {
  const storedOwner = localStorage.getItem('tunnelStatesOwner');
  return storedOwner && storedOwner !== String(owner) ? [] : Object.keys(readStates());
}

export function rememberTunnel(owner: number, id: string) {
  if (!tunnelInstances.has(id)) return;
  const states = Object.fromEntries(getSavedTunnelIds(owner).map(id => [id, 'running']));
  states[id] = 'running';
  localStorage.setItem('tunnelStatesOwner', String(owner));
  localStorage.setItem('tunnelStates', JSON.stringify(states));
}

export function recordStoppedTunnels(instances: readonly TunnelInstance[]) {
  const stopped: string[] = [];
  for (const { tunnelId, instanceId } of instances) {
    const current = tunnelInstances.get(tunnelId);
    if (current && current !== instanceId) continue;
    tunnelInstances.delete(tunnelId);
    stopped.push(tunnelId);
  }
  forgetTunnels(stopped);
}

export function forgetTunnels(ids: readonly string[]) {
  const states = readStates();
  if (!ids.some(id => id in states)) return;
  ids.forEach(id => delete states[id]);
  localStorage.setItem('tunnelStates', JSON.stringify(states));
}

const isReadyLog = (line: string) => /\bstart proxy success\b|启动成功|success for proxy/i.test(line);

export async function startManagedTunnel(tunnel: TunnelStart): Promise<'ready' | 'running' | 'already-running'> {
  const { id, name, remote, token, external } = tunnel;
  if (pendingTunnels.has(id)) throw new Error('隧道正在处理中');
  pendingTunnels.add(id);
  const cleanup: (() => void)[] = [];
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    if (await invoke<boolean>('check_frpc_status', { id })) {
      await refreshRunningTunnels();
      return 'already-running';
    }
    const instanceId = uuid();
    let finish: (status: 'ready' | 'exited' | 'timeout') => void = () => {};
    const started = new Promise<'ready' | 'exited' | 'timeout'>(resolve => { finish = resolve; });
    cleanup.push(await listen<TunnelInstance & { message: string }>('frpc-instance-log', ({ payload }) => {
      if (payload.tunnelId === id && payload.instanceId === instanceId && isReadyLog(payload.message)) finish('ready');
    }));
    cleanup.push(await listen<TunnelInstance[]>('tunnels-stopped', ({ payload }) => {
      if (payload.some(item => item.tunnelId === id && item.instanceId === instanceId)) finish('exited');
    }));
    globalLogService.appendSystemLog(`开始启动隧道 #${id} ${name}`);
    tunnelInstances.set(id, instanceId);
    try {
      await invoke('start_frpc_instance', { id, tunnelId: id, instanceId, name, remote, token, external });
    } catch (error) {
      if (tunnelInstances.get(id) === instanceId) tunnelInstances.delete(id);
      throw error;
    }
    void refreshRunningTunnels().catch(console.error);
    timer = setTimeout(() => finish('timeout'), 25000);
    const result = await started;
    const alive = result !== 'exited' && await invoke<boolean>('check_frpc_status', { id });
    if (!alive || tunnelInstances.get(id) !== instanceId) {
      if (tunnelInstances.get(id) === instanceId) tunnelInstances.delete(id);
      throw new Error('Frpc 进程已退出，请查看运行日志');
    }
    return result === 'ready' ? 'ready' : 'running';
  } finally {
    if (timer) clearTimeout(timer);
    cleanup.forEach(unlisten => unlisten());
    pendingTunnels.delete(id);
    void refreshRunningTunnels().catch(console.error);
  }
}

export async function stopManagedTunnel(id: string) {
  if (pendingTunnels.has(id)) return;
  pendingTunnels.add(id);
  try {
    await invoke('stop_frpc_instance', { id });
    tunnelInstances.delete(id);
    forgetTunnels([id]);
    globalLogService.appendSystemLog(`已停止隧道 #${id}`);
  } finally {
    pendingTunnels.delete(id);
    void refreshRunningTunnels().catch(console.error);
  }
}
