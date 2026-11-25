import { h } from 'vue';
import { NButton, useNotification } from 'naive-ui';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';

export async function notifySystem(title: string, body: string) {
  try {
    if (!await isPermissionGranted() && await requestPermission() !== 'granted') return;
    sendNotification({ title, body });
  } catch (error) {
    console.warn('发送系统通知失败：', error);
  }
}

export function useTunnelStartedNotice() {
  const notification = useNotification();
  return (title: string, address: string, copyAddress: () => void) => {
    const firstAddress = address.split('\n').find(line => line.trim())?.trim();
    notification.success({
      title,
      description: address ? `连接地址：${address}` : '隧道启动成功',
      duration: 5000,
      content: firstAddress ? () => h(NButton, { text: true, onClick: copyAddress }, () => '复制链接地址') : undefined,
    });
    void notifySystem(title, firstAddress ? `连接地址：${firstAddress}` : '隧道启动成功');
  };
}
