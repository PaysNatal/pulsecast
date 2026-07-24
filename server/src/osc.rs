//! VRChat OSC 适配 —— 把实时心率通过 OSC/UDP 推送给 VRChat。
//!
//! VRChat 在 `127.0.0.1:9000` 监听 OSC 输入。本模块订阅心率广播，
//! 按 VRChat 社区通行约定发送 avatar 参数，并可选把心率显示到 ChatBox。
//!
//! 发送的 avatar 参数（对应 `/avatar/parameters/<name>`）：
//! - `HeartRate`        : int   0..255（原始 BPM，钳制到 255）
//! - `HeartRateInt`     : int   0..255（同上，别名，兼容不同 avatar）
//! - `HeartRateFloat`   : float -1.0..1.0（bpm/127-1，便于做 0 中心映射）
//! - `HeartRatePercent` : float 0.0..1.0（bpm/255，便于直接驱动 shader/动画）
//! - `isHRConnected`    : bool  是否已连接到心率源
//! - `onesHR/tensHR/hundredsHR` : int 个/十/百位（驱动三位数字翻牌 avatar）
//!
//! 零外部依赖：手写 OSC 1.0 编码（big-endian，4 字节对齐），tokio UDP 发送。

use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;

use crate::{HrFrame, HrStatus};

/// OSC 参数值类型（本模块用到的子集）。
enum OscArg<'a> {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(&'a str),
}

/// 把一个 OSC 消息编码进 `buf`（address + typetag + args）。
fn encode_message(buf: &mut Vec<u8>, address: &str, args: &[OscArg]) {
    buf.clear();
    push_osc_string(buf, address);

    // 类型标签串，以 ',' 开头
    let mut tags = String::with_capacity(args.len() + 1);
    tags.push(',');
    for a in args {
        tags.push(match a {
            OscArg::Int(_) => 'i',
            OscArg::Float(_) => 'f',
            // OSC 1.0：bool 用 T/F 标签，无实际负载字节
            OscArg::Bool(true) => 'T',
            OscArg::Bool(false) => 'F',
            OscArg::Str(_) => 's',
        });
    }
    push_osc_string(buf, &tags);

    for a in args {
        match a {
            OscArg::Int(v) => buf.extend_from_slice(&v.to_be_bytes()),
            OscArg::Float(v) => buf.extend_from_slice(&v.to_be_bytes()),
            OscArg::Bool(_) => {} // T/F 无负载
            OscArg::Str(s) => push_osc_string(buf, s),
        }
    }
}

/// 写入 OSC 字符串：null 结尾并补齐到 4 字节边界。
fn push_osc_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    while !buf.len().is_multiple_of(4) {
        buf.push(0);
    }
}

/// 发送单个 avatar 参数。
async fn send_param(sock: &UdpSocket, target: &SocketAddr, name: &str, arg: OscArg<'_>) {
    let mut buf = Vec::with_capacity(64);
    let addr = format!("/avatar/parameters/{name}");
    encode_message(&mut buf, &addr, std::slice::from_ref(&arg));
    let _ = sock.send_to(&buf, target).await;
}

/// 启动 VRChat OSC 转发任务：订阅心率广播，按帧发送 OSC。
///
/// - `osc_addr`：VRChat OSC 监听地址（默认 `127.0.0.1:9000`）
/// - `chatbox`：是否同时把心率写到 VRChat ChatBox
pub fn spawn(tx: &broadcast::Sender<HrFrame>, osc_addr: String, chatbox: bool) {
    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        let target: SocketAddr = match osc_addr.parse() {
            Ok(a) => a,
            Err(e) => {
                log::error!("VRChat OSC 地址无效 '{osc_addr}': {e}");
                return;
            }
        };
        // 绑定任意本地端口用于发送
        let sock = match UdpSocket::bind(("0.0.0.0", 0)).await {
            Ok(s) => s,
            Err(e) => {
                log::error!("VRChat OSC UDP 绑定失败: {e}");
                return;
            }
        };
        log::info!("VRChat OSC 转发已启动 → {target}（ChatBox: {}）", chatbox);

        loop {
            match rx.recv().await {
                Ok(frame) => {
                    let connected = matches!(frame.status, HrStatus::Live);
                    send_param(&sock, &target, "isHRConnected", OscArg::Bool(connected)).await;

                    // 非 Live 状态不发送数值参数，避免 VRChat 显示 0 BPM
                    if !connected {
                        continue;
                    }
                    let bpm = frame.bpm.min(255) as i32;

                    send_param(&sock, &target, "HeartRate", OscArg::Int(bpm)).await;
                    send_param(&sock, &target, "HeartRateInt", OscArg::Int(bpm)).await;
                    send_param(
                        &sock,
                        &target,
                        "HeartRateFloat",
                        OscArg::Float((bpm as f32 / 127.0) - 1.0),
                    )
                    .await;
                    send_param(
                        &sock,
                        &target,
                        "HeartRatePercent",
                        OscArg::Float(bpm as f32 / 255.0),
                    )
                    .await;
                    send_param(&sock, &target, "isHRConnected", OscArg::Bool(connected)).await;
                    // 三位数字翻牌
                    send_param(&sock, &target, "onesHR", OscArg::Int(bpm % 10)).await;
                    send_param(&sock, &target, "tensHR", OscArg::Int((bpm / 10) % 10)).await;
                    send_param(
                        &sock,
                        &target,
                        "hundredsHR",
                        OscArg::Int((bpm / 100) % 10),
                    )
                    .await;

                    if chatbox {
                        // /chatbox/input : [string, bool(立即发送), bool(提示音)]
                        let text = if connected {
                            format!("\u{2665} {bpm} BPM")
                        } else {
                            "\u{2665} --".to_string()
                        };
                        let mut buf = Vec::with_capacity(64);
                        encode_message(
                            &mut buf,
                            "/chatbox/input",
                            &[OscArg::Str(&text), OscArg::Bool(true), OscArg::Bool(false)],
                        );
                        let _ = sock.send_to(&buf, &target).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc_string_padding() {
        let mut b = Vec::new();
        push_osc_string(&mut b, "abc"); // 3 chars + null = 4，正好对齐
        assert_eq!(b.len(), 4);
        assert_eq!(&b, b"abc\0");

        let mut b2 = Vec::new();
        push_osc_string(&mut b2, "test"); // 4 chars + null = 5 → 补到 8
        assert_eq!(b2.len(), 8);
        assert_eq!(&b2, b"test\0\0\0\0");
    }

    #[test]
    fn encode_int_message() {
        let mut b = Vec::new();
        encode_message(&mut b, "/avatar/parameters/HeartRate", &[OscArg::Int(72)]);
        // address(28 -> 补到 28? "/avatar/parameters/HeartRate"=28 chars) 每段对齐
        // 只验证整体长度为 4 的倍数，且末尾 4 字节是 72 的 be
        assert_eq!(b.len() % 4, 0);
        assert_eq!(&b[b.len() - 4..], &72i32.to_be_bytes());
    }

    #[test]
    fn encode_typetag_bool() {
        let mut b = Vec::new();
        encode_message(
            &mut b,
            "/chatbox/input",
            &[OscArg::Str("hi"), OscArg::Bool(true), OscArg::Bool(false)],
        );
        assert_eq!(b.len() % 4, 0);
    }
}
