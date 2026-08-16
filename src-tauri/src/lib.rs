// 极序排队 商户端 · Tauri 外壳
// 桌面(desktop)：多 webview(标题栏+内容) + 系统托盘 + 强制自动更新。
// 移动(Android)：全屏单 webview 加载商户后台 duitaofang.cn。
// 用 cfg(desktop)/cfg(mobile) 分两套——桌面专用 API(托盘/多窗口/更新器)在安卓不可用。

#[cfg(desktop)]
use std::sync::atomic::{AtomicI64, Ordering};
#[cfg(desktop)]
use std::sync::Arc;
#[cfg(desktop)]
use std::time::Duration;
#[cfg(desktop)]
use tauri::image::Image;
#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(desktop)]
use tauri::webview::{DownloadEvent, WebviewBuilder};
#[cfg(desktop)]
use tauri::window::WindowBuilder;
#[cfg(desktop)]
use tauri::{Listener, LogicalPosition, LogicalSize};
use tauri::Manager;
use tauri::WebviewUrl;
#[cfg(desktop)]
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
#[cfg(desktop)]
use tauri_plugin_updater::UpdaterExt;

// 启动检查更新：发现新版本 → 强制更新（弹原生框 → 下载安装 → 自动重启）。
// 不更新就退不出这个流程（点掉提示框后立即开始装），保证所有商户都在最新版。
#[cfg(desktop)]
fn check_update_and_force(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(_) => return,
        };
        if let Ok(Some(update)) = updater.check().await {
            let ver = update.version.clone();
            app.dialog()
                .message(format!(
                    "发现新版本 {}，需要更新后才能继续使用。点击确定开始更新，完成后会自动重启。",
                    ver
                ))
                .title("有新版本")
                .kind(MessageDialogKind::Info)
                .blocking_show();
            if update
                .download_and_install(|_chunk, _total| {}, || {})
                .await
                .is_ok()
            {
                app.restart();
            }
        }
    });
}

// 让窗口显示并聚焦（托盘点击 / 菜单"显示"用）
#[cfg(desktop)]
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// 托盘悬浮提示：未读消息数 + 发信人名字（最多 5 个）
#[cfg(desktop)]
fn build_tooltip(count: i64, names: Option<&serde_json::Value>) -> String {
    if count <= 0 {
        return "极序排队 · 商户端".to_string();
    }
    let who = names
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .take(5)
                .collect::<Vec<_>>()
                .join("、")
        })
        .unwrap_or_default();
    if who.is_empty() {
        format!("{} 条新消息", count)
    } else {
        format!("{} 条新消息：{}", count, who)
    }
}

#[cfg(desktop)]
const WIN_W: f64 = 1280.0;
#[cfg(desktop)]
const WIN_H: f64 = 800.0;
#[cfg(desktop)]
const TB_H: f64 = 34.0; // 标题栏高度

// 内容页(商户后台)注入：① 桌面标记；② 禁用网页式文本选择；③ 新排队/新消息原生通知 + 提示音。
#[cfg(desktop)]
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
  var lastWaiting = null, lastUnread = null, started = false, pollGeneration = 0;
  var qWaiting = 0, qUnread = 0, qNames = [];
  function authToken() { try { return (JSON.parse(localStorage.getItem('dtw_auth') || '{}') || {}).token || ''; } catch (e) { return ''; } }
  function curMid() { try { return localStorage.getItem('dtw_last_workspace_id') || ''; } catch (e) { return ''; } }
  // 共享 AudioContext：webview autoplay 限制下首次需用户手势才出声；用户一交互就 resume，
  // 之后叫号提示音才稳定可响（否则首次 ding 静默 = "没声音"）。
  var sharedAC = null;
  function getAC() {
    try {
      if (!sharedAC) { var AC = window.AudioContext || window.webkitAudioContext; if (AC) sharedAC = new AC(); }
      if (sharedAC && sharedAC.state === 'suspended') { sharedAC.resume(); }
    } catch (e) {}
    return sharedAC;
  }
  ['pointerdown', 'keydown'].forEach(function (ev) { document.addEventListener(ev, function () { getAC(); }, true); });
  function ding() {
    // 两声"叮咚"（高→低，钟鸣感）
    try {
      var c = getAC(); if (!c) return;
      function tone(freq, start, dur, peak) {
        var o = c.createOscillator(), g = c.createGain();
        o.type = 'sine'; o.frequency.value = freq; o.connect(g); g.connect(c.destination);
        var t0 = c.currentTime + start;
        g.gain.setValueAtTime(0.0001, t0);
        g.gain.exponentialRampToValueAtTime(peak, t0 + 0.015);
        g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
        o.start(t0); o.stop(t0 + dur + 0.02);
      }
      tone(988, 0.00, 0.18, 0.38); // 叮（B5）
      tone(740, 0.15, 0.52, 0.42); // 咚（F#5，余韵略长）
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
  // 任务栏闪烁(像 QQ/微信)：新单/新消息时通知原生请求"用户注意"。窗口没聚焦才闪、聚焦后系统自动停。
  function emitAttention() {
    try { var E = window.__TAURI__ && window.__TAURI__.event; if (E) E.emit('dtw://attention', {}); } catch (e) {}
  }
  function alertNew(title, body) { notify(title, body); ding(); emitAttention(); }
  // 把未读数 + 发信人名字推给原生托盘（驱动闪烁 + 悬浮看是谁）
  function emitTray(count, names) {
    try { var E = window.__TAURI__ && window.__TAURI__.event; if (E) E.emit('dtw://tray', { count: count, names: names || [] }); } catch (e) {}
  }
  // 托盘"需要关注"= 排队人数 + 未读消息；任一>0 即闪烁。号来了(qWaiting>0)就闪。
  function pushTray() { emitTray(qWaiting + qUnread, qNames); }
  function poll() {
    var generation = ++pollGeneration;
    var t = authToken(), m = curMid(); if (!t || !m) return;
    var h = { 'Authorization': 'Bearer ' + t };
    fetch('/api/b/' + m + '/queue/overview', { headers: h }).then(function (r) { return r.ok ? r.json() : null; }).then(function (d) {
      if (generation !== pollGeneration || m !== curMid()) return;
      if (!d) return;
      var w = (d.summary && typeof d.summary.waiting_count === 'number') ? d.summary.waiting_count : (typeof d.waiting_count === 'number' ? d.waiting_count : 0);
      if (lastWaiting !== null && w > lastWaiting) { alertNew('新顾客排队', '有 ' + (w - lastWaiting) + ' 位新顾客取号，去看看'); }
      lastWaiting = w;
      qWaiting = w; pushTray(); // 排队人数驱动托盘闪烁
    }).catch(function () {});
    fetch('/api/b/' + m + '/im/summary', { headers: h }).then(function (r) { return r.ok ? r.json() : null; }).then(function (d) {
      if (generation !== pollGeneration || m !== curMid()) return;
      if (!d) return;
      // 个人未读账本是真源。全店数只兼容旧后端，不能把其他店务的已读状态带给当前账号。
      var u = typeof d.current_admin_unread_count === 'number' ? d.current_admin_unread_count : (typeof d.merchant_unread_count === 'number' ? d.merchant_unread_count : 0);
      // 首次成功读取时也提示已有未读，覆盖 WebView 刚启动、WS 尚未握手完成的窗口。
      if ((lastUnread === null && u > 0) || (lastUnread !== null && u > lastUnread)) { alertNew('新消息', '有顾客给你发来消息'); }
      lastUnread = u;
      // 托盘闪烁 + 悬浮看是谁：有未读就拉一次未读会话拿发信人名字，没有就清零
      qUnread = u;
      if (u > 0) {
        fetch('/api/b/' + m + '/im/conversations?has_unread=1', { headers: h }).then(function (r) { return r.ok ? r.json() : null; }).then(function (cd) {
          var names = [];
          if (cd && cd.conversations) { names = cd.conversations.map(function (cv) { return (cv && (cv.user_name || cv.title)) || '顾客'; }).slice(0, 5); }
          qNames = names; pushTray();
        }).catch(function () { qNames = []; pushTray(); });
      } else {
        qNames = []; pushTray();
      }
    }).catch(function () {});
  }
  // 实时：连 WebSocket(/ws 订阅 merchant:<mid>)，收到任何变更就立刻核对计数(poll 内部比较，只在真新增时响+闪)。
  // 替掉 12s 轮询；断线 3s 重连；另留 60s 慢轮询兜底(WS 断线期间不漏)；切换门店重连到新 topic。
  var ws = null, wsRetry = null, wsMid = '', pollDeb = null;
  function debouncedPoll() { if (pollDeb) return; pollDeb = setTimeout(function () { pollDeb = null; poll(); }, 400); }
  function connectWS() {
    var t = authToken(), m = curMid(); if (!t || !m) return;
    wsMid = m;
    try {
      var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      ws = new WebSocket(proto + '//' + location.host + '/ws?token=' + encodeURIComponent(t) + '&topics=' + encodeURIComponent('merchant:' + m));
      ws.onmessage = function () { debouncedPoll(); };
      ws.onclose = function () { ws = null; if (wsRetry) clearTimeout(wsRetry); wsRetry = setTimeout(connectWS, 3000); };
      ws.onerror = function () { try { ws.close(); } catch (e) {} };
    } catch (e) { if (wsRetry) clearTimeout(wsRetry); wsRetry = setTimeout(connectWS, 3000); }
  }
  function start() {
    if (started) return; started = true;
    poll();
    connectWS();
    setInterval(poll, 60000); // 慢轮询兜底
    setInterval(function () { // 切换门店：mid 变 → 重置基线 + 重连到新 topic
      var m = curMid();
      if (m && m !== wsMid) { lastWaiting = null; lastUnread = null; try { if (ws) ws.close(); } catch (e) {} connectWS(); }
    }, 5000);
  }
  var wait = setInterval(function () { if (authToken() && curMid()) { clearInterval(wait); start(); } }, 3000);

  // 诊断("点不动"排查):按住 Alt 右键 → 弹出该点最顶层元素 + z-index。
  // 普通右键不触发,不打扰正常使用;只为定位"是谁盖住了侧边栏"。
  document.addEventListener('contextmenu', function (e) {
    if (!e.altKey) return;
    e.preventDefault();
    try {
      var el = document.elementFromPoint(e.clientX, e.clientY);
      if (!el) { alert('该位置没有元素'); return; }
      var cs = getComputedStyle(el);
      var info = '标签: ' + el.tagName
        + '\nclass: ' + String(el.className || '').slice(0, 140)
        + '\nz-index: ' + cs.zIndex
        + '\nposition: ' + cs.position
        + '\npointer-events: ' + cs.pointerEvents;
      alert('盖在最上面的元素:\n\n' + info);
    } catch (err) { alert('诊断失败: ' + err); }
  }, true);
})();
"#;

// 移动端注入：把登录 token + 当前商户 id 周期性交给原生（供 Kotlin 前台服务后台轮询用）。
#[cfg(mobile)]
const MOBILE_INIT_JS: &str = r#"
(function () {
  function creds() {
    try {
      var a = JSON.parse(localStorage.getItem('dtw_auth') || '{}') || {};
      return { token: a.token || '', mid: localStorage.getItem('dtw_last_workspace_id') || '' };
    } catch (e) { return { token: '', mid: '' }; }
  }
  function push() {
    var c = creds(); if (!c.token || !c.mid) return;
    try { var inv = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke; if (inv) inv('set_push_creds', { token: c.token, mid: c.mid }); } catch (e) {}
  }
  setTimeout(push, 2000); setInterval(push, 10000);
})();
"#;

// webview 把 token+mid 交给原生，写到 app 数据目录（多个候选目录都写一遍，规避 Tauri/Kotlin 路径口径差异）。
#[tauri::command]
fn set_push_creds(app: tauri::AppHandle, token: String, mid: String) {
    let json = format!(
        "{{\"token\":\"{}\",\"mid\":\"{}\"}}",
        token.replace('"', "").replace('\\', ""),
        mid.replace('"', "").replace('\\', "")
    );
    for dir in [
        app.path().app_data_dir().ok(),
        app.path().app_local_data_dir().ok(),
        app.path().app_cache_dir().ok(),
    ] {
        if let Some(d) = dir {
            let _ = std::fs::create_dir_all(&d);
            let _ = std::fs::write(d.join("push_creds.json"), &json);
        }
    }
}

#[cfg(mobile)]
fn setup_mobile(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 安卓：全屏单 webview。先加载本地恢复页 content.html（探测到后台可达再跳转），
    // 避免冷启动/网络抖动时直冲远程失败 → 永久白屏。注入脚本把 token 桥接给原生后台服务做轮询通知。
    tauri::WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::App("content.html".into()),
    )
    .title("极序排队商户端")
    .initialization_script(MOBILE_INIT_JS)
    .build()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init());
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .invoke_handler(tauri::generate_handler![set_push_creds])
        .setup(|app| {
            #[cfg(desktop)]
            setup_desktop(app)?;
            #[cfg(mobile)]
            setup_mobile(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}

#[cfg(desktop)]
fn setup_desktop(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 启动即检查更新：发现新版 → 强制更新（弹框必须更新 → 下载安装 → 重启）
    check_update_and_force(app.handle().clone());
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

            // 内容 webview（商户后台，标题栏下方）。
            // 先加载本地恢复页 content.html：它永远能渲染（本地），探测到 duitaofang.cn 可达后再
            // location.replace 跳过去；失败则自动重试 + 手动“重试”按钮——根治“远程加载失败=永久白屏”。
            window.add_child(
                WebviewBuilder::new(
                    "content",
                    WebviewUrl::App("content.html".into()),
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

            // 托盘"像 QQ/微信"那样：未读时图标闪烁 + 悬浮看是谁。
            // 内容 webview 轮询后 emit `dtw://tray` {count, names}；这里收事件 → 更新悬浮提示 + 驱动闪烁。
            let di = app.default_window_icon().unwrap();
            let normal_icon = Image::new_owned(di.rgba().to_vec(), di.width(), di.height());
            let blank_icon = Image::new_owned(
                vec![0u8; (di.width() * di.height() * 4) as usize],
                di.width(),
                di.height(),
            );
            let unread = Arc::new(AtomicI64::new(0));

            // 收事件：存未读数 + 设悬浮提示（含发信人名字）
            let h_listen = app.handle().clone();
            let unread_l = unread.clone();
            app.listen("dtw://tray", move |event| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let count = v.get("count").and_then(|x| x.as_i64()).unwrap_or(0);
                    unread_l.store(count, Ordering::Relaxed);
                    if let Some(tray) = h_listen.tray_by_id("main-tray") {
                        let _ = tray.set_tooltip(Some(build_tooltip(count, v.get("names"))));
                    }
                }
            });

            // 任务栏闪烁(像 QQ/微信)：内容 webview 在新单/新消息时 emit `dtw://attention`；
            // 窗口没聚焦才请求"用户注意"(Windows 任务栏按钮橙闪，用户点开聚焦后系统自动停)。
            let h_attn = app.handle().clone();
            app.listen("dtw://attention", move |_event| {
                if let Some(w) = h_attn.get_window("main") {
                    if !w.is_focused().unwrap_or(false) {
                        let _ = w.request_user_attention(Some(tauri::UserAttentionType::Critical));
                    }
                }
            });

            // 闪烁线程：未读>0 时图标在「正常 / 空白」间切换；归零后复位常显。
            let h_blink = app.handle().clone();
            let unread_b = unread.clone();
            std::thread::spawn(move || {
                let mut on = true;
                loop {
                    std::thread::sleep(Duration::from_millis(550));
                    let count = unread_b.load(Ordering::Relaxed);
                    if let Some(tray) = h_blink.tray_by_id("main-tray") {
                        if count > 0 {
                            on = !on;
                            let _ = tray.set_icon(Some(if on {
                                normal_icon.clone()
                            } else {
                                blank_icon.clone()
                            }));
                        } else if !on {
                            on = true;
                            let _ = tray.set_icon(Some(normal_icon.clone()));
                        }
                    }
                }
            });

    Ok(())
}
