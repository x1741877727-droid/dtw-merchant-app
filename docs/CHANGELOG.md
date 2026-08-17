# 商户端桌面程序变更记录

## 1.0.6 — 2026-08-17 18:05 CST

- 类型：修复 / 发布。
- 现象与范围：桌面商户端在切换门店或多个轮询响应乱序时，可能将旧门店的未读数带入当前门店；首次打开已有未读也不一定触发提醒。
- 根因：轮询没有请求代次和当前门店校验，且未读优先读取门店总数而非当前商户账号的未读账本。
- 修复：每轮请求按代次和门店丢弃过期响应，优先读取 `current_admin_unread_count`，首次读取已有未读时也触发提醒；版本从 1.0.5 升至 1.0.6，避免覆盖同版本安装包。
- 验证：`node scripts/version.mjs check` 与 `CARGO_TARGET_DIR=/tmp/dtw-merchant-cargo-check-20260817 cargo check --offline` 通过。
- 上线状态：待推送 `main` 触发 GitHub Actions 生成和发布 Windows 安装包；不包含 Android 构建缓存或 `.DS_Store`。
