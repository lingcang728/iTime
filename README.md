# iTime

记录你的屏幕时间，也记录 AI 替你工作的时间。

iTime 是一个使用 Tauri 2、Vue 3、TypeScript 与 Vite 构建的本地优先 Windows 桌面应用。当前交付包含首页、AI 代理、时间线、输入足迹、周报、提醒与目标、设置七个一级页面，以及账户与 Pro 预留二级页。

## 数据边界

- Tauri 桌面运行时只读 `%APPDATA%\keystats\keystats-data.json`，通过严格类型适配器提供真实日级键鼠累计、今日键位和功能组合键；不会修改 KeyStats 文件或启动项。
- KeyStats 历史没有小时/分钟桶，历史点击也无法拆分左右键。iTime 会把这些能力如实标为不可用，不生成模拟曲线或伪造 0 值。
- iTime 从启用后每 10 秒采样一次前台可执行程序与本会话输入活跃状态，写入 `%LOCALAPPDATA%\iTime\Data\activity-v1.jsonl`。接入前的应用与 AI 历史不会回填。
- 活动记录不读取或保存窗口标题、文档名、对话内容、键入内容、PID、完整可执行路径或用户名；应用身份使用本机路径的截断 SHA-256 散列。
- AI 工具前台时长来自真实进程采样，但只能作为估算，不能等同于后台代理执行时长。没有可靠证据的指标会标为估算或暂无数据。
- 普通浏览器开发模式保留明确标记的预览数据，便于视觉回归；Tauri 桌面运行时不使用这些预览记录。
- 开机自启动通过 Tauri autostart 插件读写 iTime 自己的当前用户启动项，不修改 KeyStats 或其他应用的启动设置。

## 本地运行

```powershell
npm install
npm run dev
npm test
npm run build
npm run tauri:dev
```

最终验收前再运行 `npm run tauri:build` 生成安装包。Tauri 默认窗口为 1180×760，最小尺寸为 960×680。

## 视觉测试

项目不会安装自己的 Playwright。脚本依次检查 `ITIME_PLAYWRIGHT`、系统 `playwright`、`py -m playwright` 与 `PLAYWRIGHT_BROWSERS_PATH` 对应的共享环境，并优先复用系统 Chrome 或 Edge。

```powershell
$env:ITIME_PLAYWRIGHT = 'C:\shared-python\Scripts\playwright.exe'
npm run test:visual
```

若本机没有共享环境，脚本会明确失败并说明如何设置 `ITIME_PLAYWRIGHT`，不会静默跳过。门禁覆盖七页 1280×820 布局、960×680 起的最小窗口矩阵、125%/150%/200% DPI、深色对比度、关键交互、三张 560×900 参考页与已认可基线回归。

参考结构 SSIM 在 12 CSS px 的低频布局层计算，以排除字体栅格化与文案差异；报告同时保留未经归一化的 `rawSsim`、差异像素比例和差异图。通过阈值为结构 SSIM ≥ 0.90、主要边界锚点偏差 ≤ 4 CSS px。认可基线后的逐像素回归阈值为差异比例 ≤ 0.5%，单像素颜色阈值 0.12。

## 主要目录

- `src/domain`：事件类型、区间代数与统一聚合。
- `src/providers`：时间数据提供器、输入适配边界与模拟实现。
- `src/pages`：七个一级页面与账户、Pro 二级页面。
- `src-tauri`：真实活动采集、原生图标、窗口、托盘、自启动、暂停记录、关闭事件与单实例。
- `scripts`：共享 Playwright 发现、窗口/DPI/对比度门禁与视觉差异比较。
