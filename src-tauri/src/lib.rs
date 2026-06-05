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
                // 注入桌面标记：网页据此隐藏"开启通知"网页提示、改走原生通知。
                .initialization_script("window.__DTW_DESKTOP__ = true;")
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
