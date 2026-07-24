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
            let _ = &app;
            #[cfg(not(debug_assertions))]
            {
                let webroot = app
                    .path()
                    .resource_dir()
                    .unwrap_or_else(|_| PathBuf::from("."));
                tauri::async_runtime::spawn(pulsecast_server::run_server(
                    pulsecast_server::Opts {
                        webroot,
                        port: 4567,
                        mock: false,
                        real: true,
                        vrchat: false,
                        osc_addr: "127.0.0.1:9000".to_string(),
                        chatbox: false,
                        threshold: None,
                        token: Some(token),
                    },
                ));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
