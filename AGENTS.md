# iTime 仓库工作指引

## 完成后的发布闭环（强制）

每次完成本仓库的一轮修改任务后，除非用户当前明确要求不要提交、不要推送或不要打包，否则必须在同一轮任务中依次完成：

1. 对最终源码运行与改动风险相称的验证；发布前至少执行项目的完整验证门禁（`npm run verify`；涉及 UI 时优先 `npm run verify:full`）。
2. 执行 `npm run package:release`。该命令必须强制重新构建，并从同一次构建同步覆盖 `release/iTime.exe` 与 `release/iTime_<version>_x64-setup.exe`；不得只更新其中一个，也不得复用旧产物。
3. 校验 `release` 中这两个文件都存在、修改时间属于本轮构建、与 Cargo 构建源文件 SHA-256 一致，且目录中没有第三个 EXE 发布文件。
4. 检查完整 diff，只暂存本轮范围内的**源码与配置**。`release/*.exe`、`CLAUDE.md`、`artifacts/`、`output/`、`.omo/` 等本地产物**禁止**进入 Git。
5. 将当前分支推送到其已配置的远端上游；确认远端最终 commit 与本地一致。
6. 若本轮包含正式版本发布：用本轮 `release/` 产物创建或更新 **GitHub Release**（`gh release create` / `gh release upload`），资产为上述两个 EXE；不要把安装包塞进 Git 树冒充 Release。
7. 再次确认工作树干净。只有验证、本地打包、源码提交推送（以及需要时的 GitHub Release）全部成功后，才能向用户报告任务完成。

`release/iTime.exe` 是可直接运行版本，`release/iTime_<version>_x64-setup.exe` 是安装包；二者仅作为本地与 GitHub Releases 资产，**不得**再提交进仓库。用户当前消息、系统或开发者指令与本规则冲突时，以更新且更高优先级的指令为准。
