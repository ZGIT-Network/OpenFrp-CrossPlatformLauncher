# OpenFrp Cross Platform Launcher

OpenFrp CPL 跨平台启动器

使用 rust + tauri2 + typescript 开发。

已配置 Github Action ，代码在 push 后会自动构建可执行文件。

# 开源协议

本项目以 **Apache-2.0 WITH Commons-Clause** 协议开源，携带附加条款。

### 允许

个人非商业使用

修改、分发（但必须保留许可证）

学术研究、非盈利项目

**禁止任何形式的商业使用**

# 贡献

欢迎各位大佬为本项目贡献代码，由于我个人代码能力有限，所以你会看到：

- ~~屎山代码~~
- ~~逆天函数~~
- ~~一个文件一千多行~~
- ~~忽略报错继续运行 (能跑就行)~~
- ~~警告一堆懒得管~~
- ~~被干掉的 ESLint(~~
- 重复函数
- 引入但未使用的变量或代码
- 我的一些奇怪小癖好（？

  还请各位大佬多多包涵！

# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Type Support For `.vue` Imports in TS

Since TypeScript cannot handle type information for `.vue` imports, they are shimmed to be a generic Vue component type by default. In most cases this is fine if you don't really care about component prop types outside of templates. However, if you wish to get actual prop types in `.vue` imports (for example to get props validation when using manual `h(...)` calls), you can enable Volar's Take Over mode by following these steps:

1. Run `Extensions: Show Built-in Extensions` from VS Code's command palette, look for `TypeScript and JavaScript Language Features`, then right click and select `Disable (Workspace)`. By default, Take Over mode will enable itself if the default TypeScript extension is disabled.
2. Reload the VS Code window by running `Developer: Reload Window` from the command palette.

You can learn more about Take Over mode [here](https://github.com/johnsoncodehk/volar/discussions/471).
