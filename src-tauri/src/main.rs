// 怦然 PulseCast · Tauri 应用入口
//
// 生产环境：在 setup 阶段内嵌启动 Rust 心率服务（端口 4567，见 pulsecast-server），
// 供 OBS 浏览器源（http://localhost:4567/obs）独立拉取实时心率。
// 开发环境：由 tauri.conf 的 beforeDevCommand（Node 版 dev-server.mjs）承担服务，
// 此处不重复启动，避免端口冲突。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let _ = &app; // 调试构建下 app 仅在此处被引用，避免未使用告警
            #[cfg(not(debug_assertions))]
            {
                // 打包后前端资源位于 resource_dir 根（由 frontendDist 复制而来）
                let webroot = app
                    .path()
                    .resource_dir()
                    .unwrap_or_else(|_| PathBuf::from("."));
                tauri::async_runtime::spawn(pulsecast_server::run_server(
                    pulsecast_server::Opts {
                        webroot,
                        port: 4567,
                        // 默认尝试真实 BLE 心率源；无设备/无蓝牙适配器时由服务端自动回退模拟源
                        mock: false,
                        real: true,
                        // OSC 转发由桌面端设置动态控制，默认关闭；
                        // 后续通过 Tauri 命令热切换（重启内嵌任务）。
                        vrchat: false,
                        osc_addr: "127.0.0.1:9000".to_string(),
                        chatbox: false,
                        // 阈值动作由桌面端设置动态控制，默认关闭；
                        // 后续通过 Tauri 命令热切换（重启内嵌任务）。
                        threshold: None,
                    },
                ));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
