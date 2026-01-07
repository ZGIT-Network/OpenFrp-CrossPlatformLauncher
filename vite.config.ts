import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import path from "path";
// 确保已安装 unplugin-auto-import 包
// 如果未安装，请运行: npm install unplugin-auto-import --save-dev
import AutoImport from 'unplugin-auto-import/vite'
import { NaiveUiResolver } from 'unplugin-vue-components/resolvers'
import Components from 'unplugin-vue-components/vite'

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  // Tauri 载入资源走 app:// 协议，构建时建议使用相对路径，避免生产环境静态资源路径解析异常
  base: './',
  plugins: [vue(),
    AutoImport({
      imports: [
        'vue',
        {
          'naive-ui': [
            'useDialog',
            'useMessage',
            'useNotification',
            'useLoadingBar'
          ]
        }
      ]
    }),
    Components({
      resolvers: [NaiveUiResolver()]
    })
  ],

  // macOS (WKWebView) 下偶发出现依赖未被预构建/分包导致的组件为 undefined，
  // 将 vicons 显式加入 optimizeDeps 可显著降低图标缺失概率
  optimizeDeps: {
    include: ['@vicons/ionicons5', '@vicons/tabler']
  },
  define: {
    __DEV_MODE__: JSON.stringify(process.env.NODE_ENV === 'production'),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    // Windows 环境下部分端口（例如 3003）可能会被系统策略/安全软件拦截导致 EACCES
    // 改用更常见的 Vite 默认端口段以提升兼容性
    port: 5173,
    strictPort: true,
    // 强制使用 IPv4，避免 localhost 解析到 ::1 触发监听问题
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 5174,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
}));
