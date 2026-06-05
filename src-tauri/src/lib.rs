// 极序排队 商户端 · Tauri 外壳
// 无边框窗口 + 注入自定义标题栏（拖动 / 最小化 / 关闭）。
// 窗口在 Rust 里创建（才能挂 initialization_script，让标题栏脚本在远程页 duitaofang.cn 上也注入）。

use tauri::webview::DownloadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_window_state::{StateFlags, WindowExt};

// 注入到每个页面（含远程商户后台）的自定义标题栏：
// 透明覆盖条 + 左侧可拖动品牌胶囊 + 右侧最小化/关闭；中间区域 pointer-events:none，不挡网页点击。
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
      '#__dtwbar{position:fixed;top:0;left:0;right:0;height:30px;z-index:2147483647;pointer-events:none;display:flex;align-items:center;justify-content:space-between;font-family:-apple-system,"PingFang SC",sans-serif}'
      + '#__dtwbar .drag{pointer-events:auto;margin-left:8px;height:21px;padding:0 11px;display:flex;align-items:center;gap:6px;border-radius:999px;background:rgba(33,66,46,.9);color:#f5e9c7;font-size:11px;font-weight:700;cursor:grab}'
      + '#__dtwbar .ctl{pointer-events:auto;display:flex;gap:4px;margin-right:8px}'
      + '#__dtwbar .b{width:27px;height:22px;display:flex;align-items:center;justify-content:center;border-radius:6px;background:rgba(33,66,46,.9);color:#e8e0c8;font-size:13px;cursor:pointer;line-height:1}'
      + '#__dtwbar .b:hover{background:rgba(33,66,46,1)}'
      + '#__dtwbar .b.x:hover{background:#e5534b;color:#fff}';
    document.head.appendChild(s);
    var bar = document.createElement('div');
    bar.id = '__dtwbar';
    bar.innerHTML =
      '<div class="drag" id="__dtwdrag">极序排队 · 商户端</div>'
      + '<div class="ctl"><div class="b" id="__dtwmin">—</div><div class="b x" id="__dtwclose">✕</div></div>';
    document.body.appendChild(bar);
    document.getElementById('__dtwdrag').addEventListener('mousedown', function (e) {
      if (e.button !== 0) return;
      var w = win(); if (w && w.startDragging) w.startDragging();
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
  // SPA 路由 / 跳转 duitaofang.cn 后重建
  setInterval(build, 2000);
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("极序排队 · 商户端")
                .inner_size(1280.0, 820.0)
                .min_inner_size(1024.0, 700.0)
                .resizable(true)
                .center()
                .decorations(false)
                .initialization_script(TITLEBAR_JS)
                // 下载(如"保存海报图")拦截：直接存到系统"下载"文件夹，不弹浏览器式下载条。
                .on_download(|webview, event| {
                    if let DownloadEvent::Requested { destination, .. } = event {
                        if let Ok(dir) = webview.app_handle().path().download_dir() {
                            let name = destination.file_name().map(|s| s.to_os_string());
                            let mut p = dir;
                            p.push(name.unwrap_or_else(|| std::ffi::OsString::from("极序排队-下载")));
                            *destination = p;
                        }
                    }
                    true
                })
                .build()?;
            // 还原上次窗口大小/位置（记住窗口）
            let _ = win.restore_state(StateFlags::all());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}
