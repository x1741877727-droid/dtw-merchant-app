# 极序排队 · 商户端（桌面 / 移动外壳）

商户后台已经是网页（duitaofang.cn）。本项目用 **Tauri** 把它包成原生应用：
开屏 → 全屏加载商户后台。后续叠加原生通知（叫号/新排队）+ 小票打印。

- 加载方式：**直连线上 `https://duitaofang.cn`**（改网页即更新，无需重发版）。
- 鉴权：网页用 `Authorization: Bearer` 头，跨域天然可用，包壳无需额外改造。

## 当前进度
- [x] Phase 1：Tauri Windows 壳（开屏 + 加载后台）→ GitHub Actions 出 `.exe`
- [ ] Phase 2：Windows 原生通知（叫号/新排队 弹窗 + 响铃）
- [ ] Phase 3：Windows 小票打印（ESC/POS）
- [ ] Phase 4：Capacitor 安卓壳 → `.apk`
- [ ] Phase 5：安卓 FCM 推送 + 蓝牙小票打印

## 怎么出 Windows 安装包（.exe）—— GitHub Actions 云编译
1. 在 GitHub 新建一个仓库（私有即可），例如 `dtw-merchant-app`。
2. 把**本目录(`merchant-app/`)的内容**推到该仓库的 `main` 分支：
   ```bash
   cd merchant-app
   git init && git add . && git commit -m "init merchant app"
   git branch -M main
   git remote add origin https://github.com/<你的账号>/dtw-merchant-app.git
   git push -u origin main
   ```
3. push 后，GitHub 仓库的 **Actions** 标签会自动跑 `build-merchant-app`（约 5-10 分钟）。
4. 跑完点进那次运行，底部 **Artifacts** 下载 `dtw-merchant-windows` → 里面是对应当前 `VERSION` 的安装包。
5. 在 Windows 双击安装即可（Win10/11 自带 WebView2 运行环境）。

> 也可手动触发：Actions 页 → build-merchant-app → Run workflow。

## 本地开发（可选，Mac/Win 都行，出对应平台的包）
```bash
npm install
npx tauri icon app-icon.png   # 生成图标
npm run dev                    # 本地起一个窗口预览
npm run build                  # 出当前平台安装包
```
注：Mac 上 `npm run build` 出的是 Mac app；Windows `.exe` 必须在 Windows 或上面的 CI 出。

## 版本发布

桌面程序使用独立版本线，唯一来源是 `VERSION`。准备发布时执行：

```bash
node scripts/version.mjs bump patch
node scripts/version.mjs check
git commit -am "chore(release): vX.Y.Z"
git tag -a vX.Y.Z -m "desktop vX.Y.Z"
git push origin main vX.Y.Z
```

GitHub Actions 只从 `src-tauri/tauri.conf.json` 读取版本并发布更新包；该文件由上述脚本同步，不能单独手改。
