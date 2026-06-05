// 极序排队 商户端 · Tauri 外壳
// 无边框窗口 + 注入"右上角窗口控制"(拖动柄 / 最小化 / 关闭)。
// 窗口在 Rust 创建（才能挂 initialization_script，让控制条在远程页 duitaofang.cn 也注入）。

use tauri::webview::DownloadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_window_state::{StateFlags, WindowExt};

// 注入每个页面（含远程后台）的右上角控制条：
// 只占右上角一小块（拖动柄 + 最小化 + 关闭），不挡左侧品牌与主内容。
// 拖动用 Tauri 原生 data-tauri-drag-region（比 JS startDragging 可靠）。
const TITLEBAR_JS: &str = r#"
(function () {
  if (window.self !== window.top) return;
  function win() {
    var t = window.__TAURI__;
    return (t && t.window && t.window.getCurrentWindow) ? t.window.getCurrentWindow() : null;
  }
  function build() {
    if (!document.body || document.getElementById('__dtwbar')) return;
    var s = document.createElement('style');
    s.textContent =
      '#__dtwbar{position:fixed;top:0;right:0;height:30px;z-index:2147483647;pointer-events:none;display:flex;align-items:center;gap:3px;padding:4px 6px 0 0;font-family:-apple-system,"PingFang SC",sans-serif}'
      + '#__dtwbar .e{pointer-events:auto;height:22px;display:flex;align-items:center;justify-content:center;border-radius:6px;background:rgba(33,66,46,.86);color:#d9cca0;cursor:pointer;line-height:1}'
      + '#__dtwbar .grip{width:34px;cursor:grab;font-size:13px;letter-spacing:1px}'
      + '#__dtwbar .b{width:28px;font-size:13px}'
      + '#__dtwbar .e:hover{background:rgba(33,66,46,1)}'
      + '#__dtwbar .b.x:hover{background:#e5534b;color:#fff}';
    document.head.appendChild(s);
    var bar = document.createElement('div');
    bar.id = '__dtwbar';
    bar.innerHTML =
      '<div class="e grip" id="__dtwdrag" data-tauri-drag-region>⠿</div>'
      + '<div class="e b" id="__dtwmin">—</div>'
      + '<div class="e b x" id="__dtwclose">✕</div>';
    document.body.appendChild(bar);
    // data-tauri-drag-region 已处理拖动；再挂一个 JS 兜底
    document.getElementById('__dtwdrag').addEventListener('mousedown', function (e) {
      if (e.button !== 0) return;
      var w = win(); if (w && w.startDragging) { try { w.startDragging(); } catch (_) {} }
    });
    document.getElementById('__dtwmin').addEventListener('click', function () {
      var w = win(); if (w && w.minimize) w.minimize();
    });
    document.getElementById('__dtwclose').addEventListener('click', function () {
      var w = win(); if (w && w.close) w.close();
    });
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();
  setInterval(build, 2000); // SPA 路由 / 跳转后重建
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("极序排队 · 商户端")
                .inner_size(1280.0, 820.0)
                .min_inner_size(1024.0, 700.0)
                .resizable(true)
                .center()
                .decorations(false)
                .initialization_script(TITLEBAR_JS)
                // 下载（如"保存海报图"）：存到系统"下载"文件夹，完成后弹系统通知，不弹浏览器下载条。
                .on_download(|webview, event| {
                    match event {
                        DownloadEvent::Requested { destination, .. } => {
                            if let Ok(dir) = webview.app_handle().path().download_dir() {
                                let name = destination.file_name().map(|s| s.to_os_string());
                                let mut p = dir;
                                p.push(
                                    name.unwrap_or_else(|| std::ffi::OsString::from("极序排队-下载")),
                                );
                                *destination = p;
                            }
                            true
                        }
                        DownloadEvent::Finished { path, success, .. } => {
                            if success {
                                let fname = path
                                    .as_ref()
                                    .and_then(|p| p.file_name())
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let _ = webview
                                    .app_handle()
                                    .notification()
                                    .builder()
                                    .title("已保存到下载文件夹")
                                    .body(fname)
                                    .show();
                            }
                            true
                        }
                        _ => true,
                    }
                })
                .build()?;
            let _ = win.restore_state(StateFlags::all());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}
