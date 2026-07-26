# iTime 0.1.1 修复与发布进度

更新日期：2026-07-27

执行来源：`C:\Users\15pro\Desktop\iTime-修复计划-2026-07-26.md`

当前状态：阶段 0 已完成，阶段 1 进行中

本文是当前修复工作的唯一仓库内进度入口。上一轮已完成的审计闭环已归档到
`docs/archive/ITIME-REPAIR-TODO-2026-07-18.md`。

## 阶段 0：真实基线与可恢复进度

- [x] 建立本轮仓库内 TODO，并保留上一轮已完成记录。
- [x] 分支与提交：`main` / `702d69065c727d7903a9d783e301618fe0fc6b5f`。
- [x] 远端：`origin/main` 与本地一致；远端没有 `v0.1.1`。
- [x] 开始时工作树干净，没有混入用户改动。
- [x] 工具版本：Node `24.15.0`、npm `11.12.1`、rustc/cargo `1.92.0`。
- [x] 复用全局 Playwright `G:\python\Scripts\playwright.exe`，浏览器缓存
  `G:\build_cache\playwright-browsers`；未在项目重复安装。
- [x] 基础门禁通过：25 个前端测试文件、97 项前端测试、39 项 Rust 测试、
  TypeScript/Vite、rustfmt、Clippy 全部通过。
- [x] `npm run verify:full` 真实复现失败；证据保存在本地
  `artifacts/visual/layout-report.json` 与同目录截图中。

基线失败：

1. `inputModeMotion=false`：柱状模式的实际过渡为 `300ms`，低于规范要求的
   `550ms`，不是测试误报。
2. `focusIndicators.light.weeklyTrend.activeVisible=false`：浅色主题焦点确实进入
   周趋势点，描边对比度为 `5.93:1`，但焦点目标留在滚动视口外；深色主题同项可见。

## 阶段 1：完整门禁与发布可信度（P0）

### 1.1 视觉门禁

- [x] 输入模式切换使用 `600ms` cubic-bezier 过渡。
- [x] `prefers-reduced-motion: reduce` 立即切换，并由 `inputReducedMotion` 单独覆盖。
- [x] 周趋势浅色/深色键盘焦点进入后均在滚动视口内且清晰可见。
- [x] `npm run verify:full` 在同一环境连续通过两次。

阶段 1.1 验证证据（2026-07-27）：

- 两次完整门禁均通过：26 个前端测试文件 / 99 项测试、39 项 Rust 测试、
  TypeScript/Vite、rustfmt、Clippy、交互矩阵和视觉回归。
- `inputModeMotion`、`inputReducedMotion`、`inputRangeMotion`、浅色/深色
  `weeklyTrend` 焦点断言均为 `true`。
- 根因修复：输入模式过渡从 300ms 提升为 600ms；周趋势 SVG 圆点动画不再用
  `r: inherit` 把最终半径归零，改为保持 `r` 属性的 scale 动画。

### 1.2 可复现打包

- [ ] `package:release` 强制运行 `verify:full` 并重新构建。
- [ ] 打包过程不写开始菜单、`HKCU\App Paths` 或其他用户系统入口。
- [ ] 同一构建同步生成便携版和安装包，且 `release` 中恰有两个 EXE。
- [ ] 生成 `release/release-manifest.json`，记录版本、提交、时间、文件大小和 SHA-256。
- [ ] 两个 release 文件分别与 Cargo/NSIS 同轮源产物哈希一致。

### 1.3 CI 与不可变发布

- [ ] Windows PR/main CI 覆盖 `npm ci`、`verify:full` 和生产依赖审计。
- [ ] 视觉失败证据可上传，但不包含用户运行数据。
- [ ] `v*` 发布工作流验证版本一致、干净重建、manifest 和不可变标签。
- [ ] 官网下载元数据由同一 manifest 生成或校验。

## 阶段 2：统计真实性与 Provider 隐私（P0）

- [ ] 统一指标定义，移除硬编码比较、静态微柱、固定时段和夸大语义。
- [ ] 缺少数据或比较基线时显示明确缺失状态。
- [ ] Provider 使用版本化显式授权；未授权时零扫描。
- [ ] Codex/Claude 独立开关和一次性权限说明。
- [ ] 只解析必要时间/事件元数据，不访问、保存或输出正文。
- [ ] 修复运行中区间缓存、候选选择、容量上限、淘汰和分级错误报告。
- [ ] 覆盖未授权、坏行、权限失败、文件删除、4096+ 候选和时间异常测试。

## 阶段 3：原生采集与 Windows 生命周期（P0）

- [ ] 暂停/继续通过控制通道即时生效，命令时间戳截断片段并隔离采集代次。
- [ ] 后端确认后前端才更新录制状态；失败回滚。
- [ ] 键盘事件携带采集代次，连续输入周期落盘，队列丢弃可观测。
- [ ] 写盘同步、有限重试、坏行恢复和退出刷新。
- [ ] Ctrl/Alt/Win 组合不计数，Shift 字符键计数。
- [ ] 用户界面统一称为“字符键按下次数”。
- [ ] 安装器与便携版注册所有权分离；应用启动和打包不抢写系统入口。
- [ ] COM 初始化/反初始化严格配对。
- [ ] 自启动状态刷新只读，只有用户明确切换时写入。

## 阶段 4：时间线、UI、无障碍和本地数据（P1）

- [ ] 时间线使用本地日期，默认 `00:00–24:00`，范围按钮真实生效。
- [ ] 当前时间只在范围内显示，热区按真实区间交集计算。
- [ ] 清理假按钮和竞态；异步状态覆盖 loading/empty/degraded/error/ready。
- [ ] 输入趋势图使用 roving tabindex、方向键、Home/End、Enter/Space。
- [ ] 图表提供屏幕阅读器摘要或等价表格。
- [ ] 设置页支持打开数据目录、JSON/CSV 导出、二次确认清空和保留期。
- [ ] JSONL 轮转、原子写入、损坏恢复和安全清理。
- [ ] 图标缓存键包含身份/尺寸/解析器版本，并按容量/LRU 清理。

## 阶段 5：文档、网站、依赖和许可证

- [ ] 官网从 manifest 或 Release API 获得版本、大小和 SHA-256。
- [ ] 更新隐私说明、下载链接验证、canonical/OG URL 和移动菜单行为。
- [ ] 生产依赖审计为 CI 阻断，开发依赖逐项审慎升级。
- [ ] 许可证由所有者确认；若发布前仍未确认，删除 README/网站“开源”声明。

许可证状态：等待所有者决定 MIT 或保持无许可证。

## 阶段 6：原生 QA 与 0.1.1 发布

- [ ] `npm ci`、`npm run verify:full`、生产依赖审计和全部新增测试通过。
- [ ] 真实 Tauri/WebView2 覆盖首次启动、托盘、退出、暂停/继续、键盘、
  Provider、提醒、自启动、数据管理和异常恢复。
- [ ] 安装版/便携版注册所有权矩阵通过。
- [ ] package/npm/Cargo/Tauri 版本统一为 `0.1.1`。
- [ ] `npm run package:release` 生成本轮两个 EXE 和有效 manifest。
- [ ] 完整 diff 只提交源码与配置，排除本地产物。
- [ ] 分片提交全部推送，远端分支与本地一致。
- [ ] 创建不可变 `v0.1.1` 标签和 GitHub Release，仅上传本轮两个 EXE。
- [ ] 官网重新下载资产后独立核对 SHA-256。
- [ ] 最终工作树干净。

## 约束与延期

- 不改写 `v0.1.0`。
- 不把 SQLite/WAL、签名、自动更新、跨平台和 CSP 深化混入本轮。
- 不在未确认时擅自选择许可证。
- 每个独立问题域完成后更新本文的状态、验证证据和剩余风险。
