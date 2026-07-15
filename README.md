# iTime

记录你的屏幕时间，也记录 AI 替你工作的时间。

iTime 是一个使用 Tauri 2、Vue 3、TypeScript 与 Vite 构建的本地优先桌面原型。当前交付包含首页、AI 代理、时间线、输入足迹、周报、提醒与目标、设置七个页面，以及可安装的 Windows NSIS 包。

## 数据边界

- 七页共用 `MockDataProvider` 中的统一时间事件仓库，页面不手填汇总指标。
- `LegacyKeyStatsAdapter → InputActivityProvider → iTime 标准输入模型` 是输入数据的稳定边界。
- 本轮只实现 `MockInputActivityProvider` 与迁移交互演示，不读取或修改真实 KeyStats 文件。
- 输入模型不保存文字内容、可还原文字的事件序列或原始键盘事件；密度只按分钟聚合。

## 本地运行

```powershell
npm install
npm run dev
npm test
npm run build
npm run tauri:build
```

Tauri 默认窗口为 1280×820。自动窗口矩阵选出的最小尺寸为 960×680；560×900 只用于三张历史参考页的视觉回归，不是受支持的产品窗口模式。

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
- `src/pages`：七个一级页面。
- `src-tauri`：窗口、托盘、暂停记录、关闭事件与单实例。
- `scripts`：共享 Playwright 发现、窗口/DPI/对比度门禁与视觉差异比较。
