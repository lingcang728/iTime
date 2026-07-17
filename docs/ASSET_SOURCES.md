# iTime 本地图标资产来源

所有图标均在构建时随应用打包，运行期间不会访问网络。

## UI 图标系统（2026-07-17）

自定义 3D 图标整套切自 4 张透明底图标板，经本地抠图后输出到 `src/assets/ui-icons/`（共 36 个 PNG，由 `scripts/extract_ui_icons.py` 生成），并通过 `src/data/uiIcons.ts` 映射到导航、指标卡、目标、输入足迹、AI 指标与周报等界面。

| 图标板 | 覆盖范围 |
| --- | --- |
| 图标系统 01 | 品牌 Logo、主导航、排行入口 |
| 图标系统 02 | 首页核心指标、AI 工作覆盖/杠杆/并发 |
| 图标系统 03 | 输入足迹指标、热力图/Top 键位/组合键/数据来源 |
| 图标系统 04 | 目标卡片、提醒与安静时段、本周成就与最专注一天 |

## 应用品牌图标

以下资源于 2026-07-15 获取：

| 图标 | 本地位置 | 官方来源 |
| --- | --- | --- |
| Visual Studio Code | `src/assets/apps/vscode.svg` | `https://code.visualstudio.com/assets/branding/visual-studio-code-icons.zip` 中的 `vscode.svg` |
| Google Chrome | `src/assets/apps/chrome.svg` | `https://www.google.com/chrome/static/images/chrome-logo-m100.svg` |
| ChatGPT | `src/data/appIconAssets.ts`（原始 PNG 字节的 Base64） | OpenAI 帮助中心官方应用图标 `https://images.ctfassets.net/j22is2dtoxu1/intercom-img-d177d076c9a5453052925143/49d5d812b0a6fcc20a14faa8c629d9fb/icon-ios-1024_401x.png` |
| Typeless | `src/data/appIconAssets.ts`（原始 PNG 字节的 Base64） | `https://www.typeless.com/logo_60.png` |
| YouTube | `src/data/appIconAssets.ts`（原始 PNG 字节的 Base64） | `https://www.youtube.com/s/desktop/a0b72b65/img/favicon_96x96.png` |
| Claude | `src/assets/apps/claude.svg` | Anthropic 官方 press kit 中的 `ClaudeIcon-Rounded.svg` |
| Google Antigravity | `src/data/appIconAssets.ts`（原始 PNG 字节的 Base64） | `https://www.antigravity.google/assets/image/brand/antigravity-icon__full-color.png` |

文件资源管理器与抽象分类没有稳定、可再分发的官方品牌资源，使用项目现有的 Phosphor 图标集作为语义图标。
