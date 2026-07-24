// 怦然 PulseCast · Tauri 应用入口
//
// 生产环境：在 setup 阶段内嵌启动 Rust 心率服务（端口 4567，见 pulsecast-server），
// 供 OBS 浏览器源（http://localhost:4567/obs）独立拉取实时心率。
// 开发环境：由 tauri.conf 的 beforeDevCommand（Node 版 dev-server.mjs）承担服务，
// 此处不重复启动，避免端口冲突。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(debug_assertions))]
use std::path::PathBuf;
use tauri::State;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

struct WsToken(String);

fn generate_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[tauri::command]
fn get_ws_token(token: State<'_, WsToken>) -> String {
    token.0.clone()
}

fn main() {
    let token = generate_token();

    tauri::Builder::default()
        .manage(WsToken(token.clone()))
        .invoke_handler(tauri::generate_handler![get_ws_token])
        .setup(move |app| {
            // ── 系统托盘 ──
            let settings_item =
                MenuItemBuilder::with_id("settings", "打开设置").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出怦然").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&settings_item, &quit_item])
                .build()?;

            TrayIconBuilder::with_id("pulsecast-tray")
                .icon(app.default_window_icon().cloned().unwrap())
                .tooltip("怦然 PulseCast")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.eval("window.location.href = '/settings'");
                            let _ = win.unminimize();
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.unminimize();
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // ── 内嵌心率服务（仅生产环境） ──
            #[cfg(not(debug_assertions))]
            {
                let webroot = app
                    .path()
                    .resource_dir()
                    .unwrap_or_else(|_| PathBuf::from("."));

                // 配置零感知：读取上次保存的设备名，自动重连
                let cfg = pulsecast_server::config::load().await;
                let saved_device = cfg.device_name.clone();

                let token_clone = token.clone();
                tauri::async_runtime::spawn(async move {
                    pulsecast_server::run_server(pulsecast_server::Opts {
                        webroot,
                        port: 4567,
                        mock: false,
                        real: true,
                        vrchat: cfg.vrchat.unwrap_or(false),
                        osc_addr: "127.0.0.1:9000".to_string(),
                        chatbox: cfg.chatbox.unwrap_or(false),
                        threshold: match (cfg.hr_high, cfg.hr_low) {
                            (Some(h), Some(l)) => {
                                Some(pulsecast_server::ThresholdCfg { high: h, low: l })
                            }
                            _ => None,
                        },
                        token: Some(token_clone),
                        initial_device: saved_device,
                    })
                    .await;
                });

                // 自动更新 OBS 浏览器源 URL（token 每次启动都变）
                let obs_token = token.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    let cfg = pulsecast_server::config::load().await;
                    let addr = cfg
                        .obs_ws_addr
                        .as_deref()
                        .unwrap_or("ws://localhost:4455");
                    let pwd = cfg.obs_ws_password.as_deref();
                    let url = format!(
                        "http://localhost:4567/obs?style=pill&token={}",
                        obs_token
                    );
                    let _ = pulsecast_server::obs_ws::inject(addr, pwd, &url).await;
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
