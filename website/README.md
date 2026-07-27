# iTime 官网（静态站点）

面向 Cloudflare Pages 的纯静态产品页。

## 本地预览

在仓库根目录：

```powershell
python -m http.server 4173 --directory website
```

浏览器打开 `http://127.0.0.1:4173`。

## 自动校验

```powershell
npm run verify:website
```

校验覆盖 Release API 对接、SHA-256 元数据、站内锚点、外部 HTTPS 链接、
canonical / Open Graph 信息、移动菜单行为和缓存策略。

## 部署

见 [DEPLOY.md](./DEPLOY.md)。

## 页面板块

- 产品介绍
- 功能说明
- 界面截图（演示 / 烟测示例数据）
- 隐私说明
- 最新版本更新日志
- 下载（GitHub Releases）
- SHA-256 校验值

版本、发布时间、文件名、大小、下载地址和 SHA-256 由浏览器从 GitHub 最新正式
Release API 读取，不在 HTML 中手工维护。
