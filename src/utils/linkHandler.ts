import { openUrl } from '@tauri-apps/plugin-opener';

// 已处理的链接集合，防止重复处理
const processedLinks = new WeakSet();

/**
 * 打开外部链接，使用系统默认浏览器
 * @param url 要打开的URL
 * @returns Promise
 */
export const openExternalLink = async (url: string): Promise<void> => {
  console.log(`[linkHandler] 尝试打开链接: ${url}`);
  try {
    await openUrl(url);
  } catch (err) {
    console.error('打开链接失败:', err);
  }
};

/**
 * 清除元素上可能存在的所有链接事件处理器
 * @param element 目标元素
 * @returns 新的链接元素
 */
const removeExistingHandlers = (link: HTMLAnchorElement): HTMLAnchorElement => {
  // 创建一个克隆节点来替换原节点，从而移除所有事件监听器
  const newLink = link.cloneNode(true) as HTMLAnchorElement;
  if (link.parentNode) {
    link.parentNode.replaceChild(newLink, link);
  }
  return newLink;
};

/**
 * 为元素中的所有外部链接添加点击事件处理器
 * @param element DOM元素
 */
export const setupExternalLinks = (element: HTMLElement): void => {
  const links = element.getElementsByTagName('a');
  
  for (let i = 0; i < links.length; i++) {
    const link = links[i] as HTMLAnchorElement;
    
    // 如果链接已经处理过，跳过
    if (processedLinks.has(link)) {
      continue;
    }
    
    // 检查是否是外部链接
    if (link.host && (link.host !== window.location.host || link.getAttribute('data-external') === 'true')) {
      console.log(`[linkHandler] 处理外部链接: ${link.href}`);
      
      link.setAttribute('target', '_blank');
      link.setAttribute('rel', 'noopener noreferrer');
      
      // 添加点击事件处理
      const clickHandler = (e: MouseEvent) => {
        console.log(`[linkHandler] 链接被点击: ${link.href}`);
        e.preventDefault();
        e.stopPropagation(); // 阻止事件冒泡
        
        if (link.href && link.href !== '#') {
          openExternalLink(link.href);
        }
        return false;
      };
      
      // 添加事件监听
      link.addEventListener('click', clickHandler, { once: true });
      
      // 标记为已处理
      processedLinks.add(link);
    }
  }
};

/**
 * Vue指令: v-external
 * 将应用到元素上的指令，会处理元素内所有外部链接
 */
export const externalDirective = {
  mounted(el: HTMLElement) {
    console.log('[linkHandler] v-external 指令挂载');
    setupExternalLinks(el);
  },
  updated(el: HTMLElement) {
    console.log('[linkHandler] v-external 指令更新');
    setupExternalLinks(el);
  }
}; 