# 将 iTime 官网部署到 Cloudflare Pages

本目录是**纯静态站点**，无需构建命令。推荐用 Git 连接自动部署（最省事，也不用把 Cloudflare 账号密码交给别人）。

官网源文件：

```
website/
  index.html
  styles.css
  main.js
  _headers
  assets/
  DEPLOY.md
```

---

## 推荐方式：Cloudflare Pages + GitHub（约 5 分钟）

### 1. 先把代码推到 GitHub

确认仓库 `lingcang728/iTime` 的 `main` 分支已包含 `website/` 目录。

### 2. 登录 Cloudflare

打开 [Cloudflare Dashboard](https://dash.cloudflare.com/) → 左侧 **Workers & Pages** → **Create** → **Pages** → **Connect to Git**。

### 3. 授权 GitHub

按提示授权 Cloudflare 访问你的 GitHub，并选择仓库 **`lingcang728/iTime`**。

### 4. 构建设置（关键）

| 配置项 | 填写值 |
| --- | --- |
| Project name | 使用账号下唯一名称，例如 `itime-desktop` |
| Production branch | `main` |
| **Root directory** | **`website`** |
| Framework preset | **None** |
| Build command | **留空** |
| Build output directory | **`/`** 或留空（根即静态文件所在处） |

> Cloudflare 的 Root directory 设为 `website` 后，部署根目录就是这个文件夹；输出目录填 `/` 或 `.` 均可（不同 UI 文案略有差异，以「不把上级仓库根当站点」为准）。

### 5. 保存并部署

点击 **Save and Deploy**。大约 1 分钟内会生成类似：

Cloudflare 会显示实际分配的 `https://<项目名>.pages.dev` 地址。不要把
`https://itime.pages.dev` 当作本项目地址；该域名当前属于无关站点。

之后每次 `main` 有 push，Pages 会自动重新部署。

### 6（可选）. 自定义域名

在 Pages 项目 → **Custom domains** 可绑定自己的域名。  
你当前要求使用 **免费 `*.pages.dev` 域名即可**，这一步可跳过。

---

## 备选方式 A：拖拽上传（最快试看）

1. Cloudflare Dashboard → **Workers & Pages** → **Create** → **Pages** → **Upload assets**
2. 项目名填写账号下尚未使用的唯一名称，例如 `itime-desktop`
3. 把本机 `website` 文件夹里的内容打成 zip，或直接拖入 `index.html`、`styles.css`、`main.js`、`_headers`、`assets/`
4. 部署后得到 `*.pages.dev` 链接

适合先预览；以后改内容需要重新上传。长期仍建议用 Git 连接。

---

## 备选方式 B：Wrangler CLI（本机命令行）

需要你在本机登录 Cloudflare，**不必把密码发给 AI**。

```powershell
# 1) 先检查是否已有 Wrangler，已安装就直接复用
Get-Command wrangler -ErrorAction SilentlyContinue

# 2) 已安装时登录（浏览器授权一次）
wrangler login

# 3) 在仓库根目录部署 website 文件夹
cd C:\Users\15pro\Desktop\MyProject\iTime
wrangler pages deploy website --project-name=itime-desktop
```

若本机没有 Wrangler，先按仓库全局规则搜索全局 npm 根目录和构建缓存，确认不存在
后再决定安装。首次部署会提示创建 Pages 项目，成功后终端会打印访问 URL。

若希望 AI 代你执行 CLI 部署，只需提供 **API Token**（权限：`Cloudflare Pages:Edit` + 账户读），**不要**分享账号密码。更推荐上面的 Git 连接，零密钥共享。

---

## 要不要把 Cloudflare 权限给我？

| 方式 | 是否需要给你授权 | 建议 |
| --- | --- | --- |
| GitHub 连接 Pages | 你在浏览器里授权 **Cloudflare ↔ GitHub** 即可 | **最推荐** |
| Wrangler + API Token | 可创建有限权限 Token 给我用 | 可用，但要注意 Token 泄露风险 |
| 账号密码 | **不要** | 不安全 |

**结论：** 你不需要把 Cloudflare 整站权限交给我。代码推到 GitHub 后，你在 Cloudflare 控制台点几下「Connect to Git」并设好 Root directory = `website` 即可。若你卡在某一步，把截图或报错发我，我可以继续帮你排。

---

## 部署后自检

- [ ] 打开 `https://<项目名>.pages.dev` 能看到首页
- [ ] 截图能加载（`/assets/screenshots/*.png`）
- [ ] 「下载安装包 / 便携版」指向 GitHub Releases
- [ ] SHA-256 区块显示且「复制」可用
- [ ] 手机宽度下菜单可展开

---

## 更新官网内容

1. 改 `website/index.html`（文案和页面结构）
2. 换图：替换 `website/assets/screenshots/`
3. 运行 `npm run verify:website`
4. 提交并 push 到 `main` → Pages 自动更新

发布新版本后，页面会从 GitHub 最新正式 Release API 自动读取 tag、发布时间、文件名、
大小、下载地址和 SHA-256。若 API 不可用，页面只引导用户前往 Releases，不显示未经
验证的旧哈希。
