import { openUrl } from '@tauri-apps/plugin-opener';

const listeners = new WeakMap<HTMLElement, (event: MouseEvent) => void>();

export const externalDirective = {
  mounted(element: HTMLElement) {
    const handleClick = (event: MouseEvent) => {
      if (!(event.target instanceof Element)) return;
      const link = event.target.closest('a[href]') as HTMLAnchorElement | null;
      if (!link) return;
      const url = new URL(link.href);
      if (url.origin === window.location.origin && link.dataset.external !== 'true') return;
      if (url.protocol === 'http:' || url.protocol === 'https:') {
        event.preventDefault();
        void openUrl(url.href).catch(error => console.error('打开链接失败:', error));
      }
    };
    listeners.set(element, handleClick);
    element.addEventListener('click', handleClick);
  },
  unmounted(element: HTMLElement) {
    const listener = listeners.get(element);
    if (listener) element.removeEventListener('click', listener);
    listeners.delete(element);
  },
}; 