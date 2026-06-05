// 极序排队 商户端 · Tauri 外壳
// 多 webview：顶部独立标题栏(整条可拖) + 下方商户后台内容区，互不重叠。
// 固定窗口大小、无系统边框；保存(如海报)用原生"另存为"对话框。

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::{DownloadEvent, WebviewBuilder};
use tauri::window::WindowBuilder;
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewUrl};
use tauri_plugin_dialog::DialogExt;

// 让窗口显示并聚焦（托盘点击 / 菜单"显示"用）
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

const WIN_W: f64 = 1280.0;
const WIN_H: f64 = 800.0;
const TB_H: f64 = 34.0; // 标题栏高度

// 内容页(商户后台)注入：① 桌面标记；② 禁用网页式文本选择；③ 新排队/新消息原生通知 + 提示音。
const CONTENT_INIT_JS: &str = r#"
window.__DTW_DESKTOP__ = true;
(function () {
  // ① 禁用网页式文本选择/拖拽（输入框仍可选）
  function inject() {
    if (document.getElementById('__dtw_noselect')) return;
    if (!document.head && !document.documentElement) return;
    var s = document.createElement('style');
    s.id = '__dtw_noselect';
    s.textContent =
      'html{-webkit-user-select:none;user-select:none;-webkit-touch-callout:none}'
      + 'input,textarea,[contenteditable],[contenteditable="true"]{-webkit-user-select:text!important;user-select:text!important}'
      + 'img,a{-webkit-user-drag:none}';
    (document.head || document.documentElement).appendChild(s);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', inject);
  inject();
  setInterval(inject, 3000);

  // ② 新排队 / 新消息 → 原生系统通知 + 提示音（桌面端轮询，复用后台已有接口）
  var lastWaiting = null, lastUnread = null, started = false;
  function authToken() { try { return (JSON.parse(localStorage.getItem('dtw_auth') || '{}') || {}).token || ''; } catch (e) { return ''; } }
  function curMid() { try { return localStorage.getItem('dtw_last_workspace_id') || ''; } catch (e) { return ''; } }
  function ding() {
    try {
      var AC = window.AudioContext || window.webkitAudioContext; if (!AC) return;
      var c = new AC(), o = c.createOscillator(), g = c.createGain();
      o.type = 'sine'; o.frequency.value = 880; o.connect(g); g.connect(c.destination);
      g.gain.setValueAtTime(0.0001, c.currentTime);
      g.gain.exponentialRampToValueAtTime(0.35, c.currentTime + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, c.currentTime + 0.35);
      o.start(); o.stop(c.currentTime + 0.36);
    } catch (e) {}
  }
  function notify(title, body) {
    try {
      var N = window.__TAURI__ && window.__TAURI__.notification; if (!N) return;
      N.isPermissionGranted().then(function (g) {
        if (g) { N.sendNotification({ title: title, body: body }); }
        else { N.requestPermission().then(function (p) { if (p === 'granted') N.sendNotification({ title: title, body: body }); }); }
      });
    } catch (e) {}
  }
  function alertNew(title, body) { notify(title, body); ding(); }
  function poll() {
    var t = authToken(), m = curMid(); if (!t || !m) return;
    var h = { 'Authorization': 'Bearer ' + t };
    fetch('/api/b/' + m + '/queue/overview', { headers: h }).then(function (r) { return r.ok ? r.json() : null; }).then(function (d) {
      if (!d) return;
      var w = (d.summary && typeof d.summary.waiting_count === 'number') ? d.summary.waiting_count : (typeof d.waiting_count === 'number' ? d.waiting_count : 0);
      if (lastWaiting !== null && w > lastWaiting) { alertNew('新顾客排队', '有 ' + (w - lastWaiting) + ' 位新顾客取号，去看看'); }
      lastWaiting = w;
    }).catch(function () {});
    fetch('/api/b/' + m + '/im/summary', { headers: h }).then(function (r) { return r.ok ? r.json() : null; }).then(function (d) {
      if (!d) return;
      var u = typeof d.merchant_unread_count === 'number' ? d.merchant_unread_count : 0;
      if (lastUnread !== null && u > lastUnread) { alertNew('新消息', '有顾客给你发来消息'); }
      lastUnread = u;
    }).catch(function () {});
  }
  function start() { if (started) return; started = true; poll(); setInterval(poll, 12000); }
  var wait = setInterval(function () { if (authToken() && curMid()) { clearInterval(wait); start(); } }, 3000);
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 固定大小、不可调、无边框、居中
            let window = WindowBuilder::new(app, "main")
                .title("极序排队 · 商户端")
                .inner_size(WIN_W, WIN_H)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .center()
                .build()?;

            // 顶部标题栏 webview（本地 titlebar.html）
            window.add_child(
                WebviewBuilder::new("titlebar", WebviewUrl::App("titlebar.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(WIN_W, TB_H),
            )?;

            // 内容 webview（商户后台，标题栏下方）
            window.add_child(
                WebviewBuilder::new(
                    "content",
                    WebviewUrl::External("https://duitaofang.cn".parse().unwrap()),
                )
                // 注入桌面标记 + 禁用网页式文本选择（native app 不该能拖蓝高亮，输入框除外）。
                .initialization_script(CONTENT_INIT_JS)
                // 下载(如"保存海报图")：弹原生"另存为"对话框，用户选位置/文件名后保存。
                .on_download(|webview, event| {
                    if let DownloadEvent::Requested { destination, .. } = event {
                        let suggested = destination
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "极序排队-海报.png".to_string());
                        let picked = webview
                            .app_handle()
                            .dialog()
                            .file()
                            .set_file_name(suggested)
                            .add_filter("图片", &["png", "jpg", "jpeg"])
                            .blocking_save_file();
                        match picked {
                            Some(fp) => {
                                if let Ok(pb) = fp.into_path() {
                                    *destination = pb;
                                }
                                true
                            }
                            None => false, // 用户取消 → 不保存
                        }
                    } else {
                        true
                    }
                }),
                LogicalPosition::new(0.0, TB_H),
                LogicalSize::new(WIN_W, WIN_H - TB_H),
            )?;

            // 系统托盘：图标 + 菜单(显示/退出)，左键点击显示主界面。
            // 后续接"新排队/新消息"时，在此更新托盘角标 + 悬浮列出未读发信人。
            let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("极序排队 · 商户端")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}
