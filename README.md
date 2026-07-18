# iTime

本地优先的 Windows 桌面应用：记录前台屏幕时间，并区分 AI 工具前台活动。  
栈：**Tauri 2 · Vue 3 · TypeScript · Vite · Rust**。

当前版本 `0.1.0`。可执行包见 [GitHub Releases](https://github.com/lingcang728/iTime/releases)，不要把 `release/*.exe` 提交进 Git。

---

## 目录

- [架构速览](#架构速览)
- [数据契约](#数据契约)
- [仓库结构](#仓库结构)
- [环境要求](#环境要求)
- [开发](#开发)
- [验证与视觉门禁](#验证与视觉门禁)
- [打包与发布](#打包与发布)
- [窗口与运行时约定](#窗口与运行时约定)

---

## 架构速览

| 层 | 路径 | 职责 |
| --- | --- | --- |
| UI 页面 | `src/pages` | 七个一级页：首页、AI 代理、时间线、输入足迹、周报、目标、设置；另有账户 / Pro 预留 |
| 领域模型 | `src/domain` | 事件类型、区间代数、指标聚合、目标模型 |
| 适配层 | `src/providers` | KeyStats 只读适配、活动数据适配、浏览器预览数据 |
| 状态 | `src/stores` | 主题、持久化、数据可用性等前端状态 |
| 桌面壳 | `src-tauri` | 前台采样、JSONL 落盘、原生图标、托盘、自启动、单实例、关闭流程 |
| 脚本 | `scripts` | 完整打包、共享 Playwright 视觉门禁、截图对比 |

浏览器 `npm run dev` 可走预览数据做 UI 开发；**Tauri 桌面运行时不使用预览记录**，只接本机真实数据源。

---

## 数据契约

开发时务必遵守这些边界，避免“假数据”或越权读写：

| 源 | 路径 / 行为 | 规则 |
| --- | --- | --- |
| KeyStats | `%APPDATA%\keystats\keystats-data.json` | **只读**；严格类型适配。提供日级键鼠累计、今日键位、功能组合键。不修改文件，不改 KeyStats 启动项。 |
| 活动采样 | `%LOCALAPPDATA%\iTime\Data\activity-v1.jsonl` | 启用后每 **10s** 采样前台可执行程序与本会话输入活跃。接入前历史**不回填**。 |
| 隐私 | — | 不读不写窗口标题、文档名、对话/键入内容、PID、完整可执行路径、用户名。应用身份用本机路径截断 **SHA-256**。 |
| AI 前台时长 | 进程采样 | 仅估算，**不等于**后台 agent 执行时长。无可靠证据的指标标为估算或暂无数据。 |
| KeyStats 局限 | — | 无小时/分钟桶；历史点击无法拆左右键。对应能力标为不可用，**禁止**伪造 0 或模拟曲线。 |
| 自启动 | Tauri autostart | 只管理 iTime 自己的当前用户启动项。 |

---

## 仓库结构

```text
iTime/
├── src/                 # Vue 前端
│   ├── pages/           # 路由页面
│   ├── components/      # UI 组件
│   ├── domain/          # 纯领域逻辑（优先单测）
│   ├── providers/       # 数据源适配
│   ├── stores/          # 前端状态
│   ├── styles/          # design tokens 与页面样式
│   └── platform/        # 桌面桥接（autostart 等）
├── src-tauri/           # Rust / Tauri
│   └── src/             # 采集、图标、设置、命令
├── scripts/             # package-release、visual-test、compare-visual
├── tests/visual/baseline/  # 已认可视觉基线（需版本化）
├── docs/                # 资产来源、修复清单等工程文档
├── release/             # 本地打包输出（gitignore，勿提交）
└── package.json
```

本地工具目录（`.claude/`、`.omo/`、`artifacts/`、`output/` 等）与 `CLAUDE.md` 均已忽略，仅留本机。

---

## 环境要求

- **Node.js** 22+（与当前 lockfile / Vite 7 匹配）
- **Rust** stable + Windows 桌面目标（MSVC）
- **WebView2**（Windows 自带或运行时）
- 可选：共享 **Playwright**（视觉门禁，见下；项目**不**自带 Playwright 依赖）

```powershell
npm install
```

---

## 开发

```powershell
# 浏览器 UI（含明确标记的预览数据）
npm run dev

# 桌面壳 + 真实数据路径
npm run tauri:dev

# 单元测试（Vitest）
npm test

# 前端 typecheck + production build
npm run build
```

常用脚本：

| 命令 | 说明 |
| --- | --- |
| `npm run dev` | Vite，端口 `1420` |
| `npm run tauri:dev` | Tauri 开发窗口 |
| `npm test` / `npm run test:watch` | 前端单测 |
| `npm run verify` | 单测 + 前端 build + `cargo fmt/clippy/test` |
| `npm run verify:full` | `verify` + 视觉门禁 |
| `npm run package:release` | 完整验证后强制重打包到 `release/` |
| `npm run tauri:build` | 底层 Tauri 构建（一般走 `package:release`） |

---

## 验证与视觉门禁

### 逻辑 / 编译门禁

```powershell
npm run verify
```

覆盖：Vitest、`vue-tsc` + Vite build、Rust `fmt --check`、`clippy -D warnings`、`cargo test`。

### 视觉门禁

不安装项目内 Playwright。脚本按序查找：

1. 环境变量 `ITIME_PLAYWRIGHT`
2. 系统 `playwright`
3. `py -m playwright`
4. `PLAYWRIGHT_BROWSERS_PATH` 共享浏览器缓存  

并优先复用系统 **Chrome / Edge**。未配置时**失败并提示**，不静默跳过。

```powershell
$env:ITIME_PLAYWRIGHT = 'C:\path\to\playwright.exe'
npm run test:visual
```

门禁范围概要：

- 七页 `1280×820` 布局
- 从 `960×680` 起的最小窗口矩阵
- 125% / 150% / 200% DPI
- 深色对比度与关键交互
- 参考页结构对比 + 已认可基线回归

阈值（摘要）：

- 参考结构 SSIM：在 12 CSS px 低频布局层计算，**≥ 0.90**；主要边界锚点偏差 **≤ 4 CSS px**
- 基线回归：差异比例 **≤ 0.5%**，单像素颜色阈值 **0.12**
- 报告保留 `rawSsim`、差异像素比例与差异图

手动对比辅助：`npm run visual:compare`。

---

## 打包与发布

### 本地产物

```powershell
npm run package:release
```

行为：

1. 先跑 `npm run verify`，失败则拒绝打包  
2. 校验 `package.json` / `tauri.conf.json` / Cargo 版本一致  
3. 强制 `tauri build`，拒绝复用旧 EXE  
4. 同步写入（且仅保留这两个文件）：
   - `release/iTime.exe` — 可直接运行  
   - `release/iTime_<version>_x64-setup.exe` — NSIS 安装包  

`release/` **整目录 gitignore**，仅作本机输出。

### GitHub Release（正式分发）

仓库的 Releases 页才是安装包入口；历史上把 EXE 塞进 Git 树**不会**出现在 Releases。

在已推送的 `main` 上创建示例：

```powershell
gh release create "v0.1.0" `
  "release/iTime.exe" `
  "release/iTime_0.1.0_x64-setup.exe" `
  --title "iTime v0.1.0" `
  --notes "Release notes here."
```

上传前确认两个文件的修改时间与 SHA-256 属于本轮 `package:release` 构建。

---

## 窗口与运行时约定

| 项 | 值 |
| --- | --- |
| 默认窗口 | 1180 × 760 |
| 最小尺寸 | 960 × 680 |
| 装饰 | 无系统边框（自定义壳） |
| 标识符 | `com.itime.desktop` |
| 安装包 | NSIS，`currentUser` |

---

## 相关文档

- `docs/ASSET_SOURCES.md` — 应用图标与 Phosphor 资产来源  
- `docs/ITIME-REPAIR-TODO.md` — 修复 / 债项清单  
- `AGENTS.md` — 本仓库自动化代理的发布与提交约定  

---

## License

Private / unpublished unless otherwise stated.
