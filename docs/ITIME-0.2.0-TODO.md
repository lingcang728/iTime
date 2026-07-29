# iTime 0.2.0 实施与发布清单

## 目标

- [x] Windows 本地图标解析 v3：快捷方式、包资源、EXE/DLL/ICO 资源优先，拒绝通用默认图标。
- [x] 单一“AI Agent 编程工具”授权，旧版授权不静默扩大。
- [x] Cursor、Antigravity、Codex、Claude Code、OpenCode、Grok Build、Hermes、OpenClaw 适配器注册表。
- [x] 匿名硬件、性能和每日工具汇总遥测。
- [x] 独立私有 `iTime-observability` Worker、D1 与开发者看板源码。
- [x] GitHub Release 签名自动更新、更新前数据刷新和 `latest.json` 源码与发布脚本。
- [ ] 应用、安装包、Tag 与 Release 统一为 `0.2.0`（源码、云端观测栈与本地产物已统一，等待发布 Tag/Release）。

## 当前外部条件

- [x] GitHub CLI 已登录，公开仓库 `lingcang728/iTime` 可写。
- [x] Cloudflare OAuth 与 Account 已授权，Worker、D1 和 Pages 命令行可写。
- [x] 私有 GitHub 仓库 `lingcang728/iTime-observability` 已创建并推送。
- [x] Cloudflare D1、Worker、Pages 与全站私有认证已部署；当前 OAuth 没有 `Access: Apps and Policies Write`，因此先使用 Pages 顶层中间件与 Windows 凭据管理器的开发者密码实现 fail-closed，且保留 Cloudflare Access 邮箱头兼容。
- [x] Tauri updater 签名密钥：已轮换，公钥写入配置，私钥与密码写入 GitHub Secrets，并保留 DPAPI 加密的离线备份。

## 验证与发布

- [x] 相关前端与 Rust 单元测试。
- [x] `npm run verify:full`。
- [x] `npm run package:release`。
- [x] `release/iTime.exe` 与 `release/iTime_0.2.0_x64-setup.exe` 同源同轮构建。
- [x] 本机安装副本与桌面快捷方式同步。
- [x] 源码提交并推送到 `origin/main`。
- [ ] `v0.2.0` GitHub Release 包含两个 EXE 和 `latest.json`。
- [ ] 重新下载远端资产并校验哈希、版本、URL 与 updater 签名。
