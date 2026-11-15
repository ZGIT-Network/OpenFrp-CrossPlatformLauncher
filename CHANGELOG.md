# 更新日志

## v0.9.2

狠狠的修，这次把登录和网络检测的老毛病一起收拾了喵~

本次更新内容如下：

- 修复了 NatayarkID OAuth 登录失败的问题：回调参数改为按服务端要求通过 URL query 传递。
- 修复了使用 Authorization 手动登录时始终提示“无效的 Authorization”的问题。
- 修复了 OAuth 登录流程中同一个 code 被前端/深链重复消费导致随机失败的问题。
- 修复了 OAuth 回调错误信息被“未找到 Authorization”覆盖的问题，现在会显示服务端真实原因。
- 统一 OpenFrp API 地址为 `api.openfrp.net`，并新增 `ofapi.naids.com` 作为备用地址自动回退。
- 清理了已失效的 `of-dev-api.bfsea.com` 地址及相关配置。
- 更新 Natayark 人机验证服务地址为 `cv.naids.com`。
- 修复了网络检测页因 CORS 误报“CORS限制或网络问题”的问题，服务检测改由后端请求。
- 修复了首页滚动到底部时最后一行及底部边距被裁切的问题。
- 统一了滚动条样式与位置，隐藏原生滚动条箭头并使用细窄圆角样式。
- 修复了登录凭证失效时未正确清理本地登录态的问题。
- 优化了粘贴 Authorization 时的首尾空白处理。

**Full Changelog**: https://github.com/ZGIT-Network/OpenFrp-CrossPlatformLauncher/compare/v0.9.1...v0.9.2
