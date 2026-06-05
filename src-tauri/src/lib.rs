// 极序排队 商户端 · Tauri 外壳
// Phase 1：开屏(../dist/index.html) → 加载线上商户后台 duitaofang.cn。
// 后续 Phase 2/3 在此注册原生命令(系统通知 / 小票打印)。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}
