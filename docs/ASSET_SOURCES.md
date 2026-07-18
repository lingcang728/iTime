# iTime 本地图标资产来源

所有图标均在构建时随应用打包，运行期间不会访问网络。

## UI 图标系统（2026-07-18）

导航、指标、目标、输入足迹与周报统一使用随应用打包的 Phosphor 线性图标，尺寸与 `regular` weight 由对应组件约束。品牌标志使用主题感知的内联时钟 SVG；界面不再包含生成式 3D PNG 图标。

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
