import { ref } from 'vue';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { errorMessage } from '@/utils/apiClient';

export interface UpdateInfo {
  title: string;
  latest: string;
  msg: string;
}

const installing = ref(false);

export function useAppUpdates() {
  const message = useMessage();
  async function installUpdate(version: string) {
    if (installing.value) return;
    installing.value = true;
    const progress = message.loading('正在下载更新...', { duration: 0 });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<number>('update-progress', ({ payload }) => {
        progress.content = `正在下载更新... ${Math.round(payload)}%`;
      });
      await invoke('download_and_install_update', { version });
      message.success('更新已安装，重启应用后生效');
    } catch (error) {
      message.error(`更新失败：${errorMessage(error)}`);
    } finally {
      unlisten?.();
      progress.destroy();
      installing.value = false;
    }
  }
  return { installing, installUpdate };
}
