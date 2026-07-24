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
    // 使用 OS CSPRNG 生成 128-bit 随机 token（不可预测）
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).expect("OS CSPRNG 不可用");
    buf.iter().map(|b| format!("{:02x}", b)).collect()
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
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("tauri.conf.json 缺少默认窗口图标"),
                )
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
                let port = cfg.port.unwrap_or(4567);
                let osc_addr = cfg.osc_addr.clone().unwrap_or_else(|| "127.0.0.1:9000".to_string());
                tauri::async_runtime::spawn(async move {
                    pulsecast_server::run_server(pulsecast_server::Opts {
                        webroot,
                        port,
                        mock: false,
                        real: true,
                        vrchat: cfg.vrchat.unwrap_or(false),
                        osc_addr,
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

                // 自动更新 OBS 浏览器源 URL（轮询等待服务启动，替代硬编码 sleep）
                let obs_token = token.clone();
                let obs_port = port;
                tauri::async_runtime::spawn(async move {
                    // 轮询等待服务就绪（最多 15 秒）
                    for _ in 0..30 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if tokio::net::TcpStream::connect(format!("127.0.0.1:{obs_port}"))
                            .await
                            .is_ok()
                        {
                            break;
                        }
                    }
                    let cfg = pulsecast_server::config::load().await;
                    let addr = cfg
                        .obs_ws_addr
                        .as_deref()
                        .unwrap_or("ws://localhost:4455");
                    let pwd = cfg.obs_ws_password.as_deref();
                    let url = format!(
                        "http://localhost:{obs_port}/obs?style=pill&token={}",
                        obs_token
                    );
                    match pulsecast_server::obs_ws::inject(addr, pwd, &url).await {
                        Ok(r) => log::info!("[pulsecast] OBS 注入: {}", r.message),
                        Err(e) => log::warn!("[pulsecast] OBS 注入失败: {e:#}"),
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
