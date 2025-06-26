// Tauri 2.x window effect helper (高斯模糊/亚克力/透明等)
// 依赖 @tauri-apps/api >= 2.x
import { appWindow } from '@tauri-apps/api/window'

/**
 * 获取当前平台（win32/darwin/linux/unknown）
 */
function getPlatform(): string {
  // Tauri 2.x 推荐 window.__TAURI__.platform
  if (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.platform) {
    return window.__TAURI__.platform;
  }
  // fallback: UA 判断
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('windows')) return 'win32';
  if (ua.includes('mac')) return 'darwin';
  if (ua.includes('linux')) return 'linux';
  return 'unknown';
}

/**
 * 设置窗口视觉特效（兼容多平台，自动选择最佳效果）
 * @param enable 是否启用特效
 */
export async function setWindowBlurEffect(enable: boolean) {
  try {
    const os = getPlatform();
    if (enable) {
      if (os === 'win32') {
        // Windows 11/10: acrylic > blur > transparent
        await appWindow.setEffect?.('acrylic').catch(async () => {
          await appWindow.setEffect?.('blur').catch(async () => {
            await appWindow.setEffect?.('transparent').catch(() => {});
          });
        });
      } else if (os === 'darwin') {
        // macOS: vibrancy > blur > transparent
        await appWindow.setEffect?.('vibrancy').catch(async () => {
          await appWindow.setEffect?.('blur').catch(async () => {
            await appWindow.setEffect?.('transparent').catch(() => {});
          });
        });
      } else if (os === 'linux') {
        // Linux: 仅支持 transparent
        await appWindow.setEffect?.('transparent').catch(() => {});
      } else {
        // 其他未知平台
        await appWindow.setEffect?.('transparent').catch(() => {});
      }
    } else {
      // 关闭特效，恢复为普通窗口
      await appWindow.setEffect?.('solid').catch(() => {});
    }
  } catch (e) {
    console.warn('setWindowBlurEffect failed:', e);
  }
}

// 可扩展：支持自定义参数、不同平台不同效果
