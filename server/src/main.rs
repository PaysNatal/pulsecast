//! 怦然 PulseCast · 内嵌服务独立运行入口
//!
//! 开发时可直接替代 Node 版 dev-server.mjs：
//!   cargo run -p pulsecast-server -- --webroot .
//! OBS 浏览器源填 http://localhost:4567/obs 即可。

use clap::Parser;
use pulsecast_server::run_server;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pulsecast-server", about = "怦然 PulseCast 内嵌心率服务")]
struct Cli {
    /// 前端文件根目录（含 ftue.html / settings.html / obs-overlay.html / design-tokens.css）
    #[arg(long, default_value = ".")]
    webroot: String,

    /// 监听端口
    #[arg(long, default_value_t = 4567)]
    port: u16,

    /// 强制模拟心率源（无蓝牙环境）
    #[arg(long)]
    mock: bool,

    /// 启用真实 BLE 读取（需 --features ble 编译）
    #[arg(long)]
    real: bool,

    /// 启用 VRChat OSC 转发（心率 → avatar 参数）
    #[arg(long)]
    vrchat: bool,

    /// VRChat OSC 监听地址
    #[arg(long, default_value = "127.0.0.1:9000")]
    osc_addr: String,

    /// 同时把心率写入 VRChat ChatBox
    #[arg(long)]
    chatbox: bool,

    /// 心率高阈值（BPM 超过触发 High 动作）；需配合 --hr-low 同时生效
    #[arg(long)]
    hr_high: Option<u32>,

    /// 心率低阈值（BPM 低于触发 Low 动作）；需配合 --hr-high 同时生效
    #[arg(long)]
    hr_low: Option<u32>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();
    run_server(pulsecast_server::Opts {
        webroot: PathBuf::from(cli.webroot),
        port: cli.port,
        mock: cli.mock,
        real: cli.real,
        vrchat: cli.vrchat,
        osc_addr: cli.osc_addr,
        chatbox: cli.chatbox,
        threshold: match (cli.hr_high, cli.hr_low) {
            (Some(h), Some(l)) => Some(pulsecast_server::ThresholdCfg { high: h, low: l }),
            _ => None,
        },
    })
    .await;
}
