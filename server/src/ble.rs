//! 真实 BLE 心率读取（feature `ble`）
//!
//! 标准蓝牙心率服务 / 特征：
//!   Service       0x180D (Heart Rate)
//!   Characteristic 0x2A37 (Heart Rate Measurement)
//!
//! 设计要点：
//! - 依靠**广播**中的 services 预筛心率设备（无需先连接每个设备），命中后再
//!   connect + discover_services 校验 0x2A37，避免盲目连接附近所有设备。
//! - `run_hr` 是一个**长生命周期热源**：设备断开后自动重连；当调用方通过
//!   `matcher_rx` 切换目标设备时，立即跳出并重新扫描，实现运行时热切换。
//! - 仅当系统无蓝牙适配器（致命）时返回 Err，由上层回退模拟源。

use anyhow::{Context, Result};
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,
};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use crate::{DeviceInfo, HrFrame, HrStatus};
use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::sync::watch;
use uuid::Uuid;

const HR_SERVICE: Uuid = Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);
const HR_MEASUREMENT: Uuid = Uuid::from_u128(0x0000_2a37_0000_1000_8000_0080_5f9b_34fb);
/// 扫描 / 重连等待上限；超时则提示未找到设备并重试
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);

/// 扫描到的设备摘要（供前端展示）
#[derive(Clone, Serialize)]
pub struct ScanDevice {
    pub name: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<i16>,
}

/// 扫描附近心率设备（默认 ~5s）。返回按名称去重的列表。
pub async fn scan(timeout: Duration) -> Result<Vec<ScanDevice>> {
    let manager = Manager::new().await.context("创建 BLE Manager 失败")?;
    let adapters = manager.adapters().await.context("枚举蓝牙适配器失败")?;
    let central = adapters
        .into_iter()
        .next()
        .context("未发现蓝牙适配器（请开启系统蓝牙）")?;

    central
        .start_scan(ScanFilter::default())
        .await
        .context("开始扫描失败")?;
    let mut events = central.events().await?;
    let mut out: Vec<ScanDevice> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let deadline = tokio::time::Instant::now() + timeout;

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(800), events.next()).await {
            Ok(Some(CentralEvent::DeviceDiscovered(id))) => {
                if let Ok(p) = central.peripheral(&id).await {
                    if let Ok(Some(props)) = p.properties().await {
                        if advertises_hr(&props) {
                            let name = props
                                .local_name
                                .clone()
                                .unwrap_or_else(|| id.to_string());
                            if seen.insert(name.clone()) {
                                out.push(ScanDevice {
                                    name,
                                    id: id.to_string(),
                                    rssi: props.rssi,
                                });
                            }
                        }
                    }
                }
            }
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    let _ = central.stop_scan().await;
    Ok(out)
}

/// 启动 BLE 心率源：扫描 → 连接 → 订阅 0x2A37 → 解析并广播；断线自动重连；
/// `matcher_rx` 改变（用户切换设备）时立即重连到新目标。
pub async fn run_hr(
    tx: broadcast::Sender<HrFrame>,
    current: std::sync::Arc<Mutex<HrFrame>>,
    mut matcher_rx: watch::Receiver<Option<String>>,
) -> Result<()> {
    let manager = Manager::new().await.context("创建 BLE Manager 失败")?;

    'outer: loop {
        let matcher = (*matcher_rx.borrow()).clone();
        let adapters = manager.adapters().await.context("枚举蓝牙适配器失败")?;
        let central = adapters
            .into_iter()
            .next()
            .context("未发现蓝牙适配器（请开启系统蓝牙）")?;

        central
            .start_scan(ScanFilter::default())
            .await
            .context("开始扫描失败")?;
        let mut events = central.events().await?;

        let mut target: Option<(Peripheral, Characteristic)> = None;
        while target.is_none() {
            tokio::select! {
                ev = events.next() => {
                    match ev {
                        Some(CentralEvent::DeviceDiscovered(id)) => {
                            if let Ok(p) = central.peripheral(&id).await {
                                if let Ok(Some(props)) = p.properties().await {
                                    if advertises_hr(&props)
                                        && p.connect().await.is_ok()
                                    {
                                        let _ = p.discover_services().await;
                                        let chars = p.characteristics();
                                        let has_hr = chars.iter().any(|c| c.uuid == HR_MEASUREMENT);
                                        let matches = matcher.as_ref().is_none_or(|m| {
                                            let n = props.local_name.as_deref().unwrap_or("");
                                            n.contains(m.as_str()) || id.to_string().contains(m.as_str())
                                        });
                                        if has_hr && matches {
                                            let ch = chars.into_iter().find(|c| c.uuid == HR_MEASUREMENT).unwrap();
                                            target = Some((p, ch));
                                        } else {
                                            let _ = p.disconnect().await;
                                        }
                                    }
                                }
                            }
                        }
                        Some(_) => continue,
                        None => break,
                    }
                }
                // 用户切换设备 / 关闭蓝牙源 → 立即重扫（或交由外层退出）
                _ = matcher_rx.changed() => {
                    let _ = central.stop_scan().await;
                    continue 'outer;
                }
                _ = tokio::time::sleep(SCAN_TIMEOUT) => break,
            }
        }
        let _ = central.stop_scan().await;

        let (periph, chr) = match target {
            Some(t) => t,
            None => anyhow::bail!("未找到匹配的心率设备（请先配对并开启广播）"),
        };

        if periph.subscribe(&chr).await.is_err() {
            anyhow::bail!("订阅心率特征失败");
        }
        let mut notifs = periph.notifications().await?;

        // 持续接收心率通知，直到断开或切换设备
        loop {
            tokio::select! {
                n = notifs.next() => {
                    match n {
                        Some(n) if n.uuid == HR_MEASUREMENT => {
                            if let Some(bpm) = parse_hr(&n.value) {
                                let props = periph.properties().await.ok().flatten();
                                let name = props
                                    .as_ref()
                                    .and_then(|p| p.local_name.clone())
                                    .unwrap_or_else(|| "心率设备".to_string());
                                let rssi = props.and_then(|p| p.rssi);
                                // 断连预测：RSSI 持续低于 -80 时提前警告
                                let msg = match rssi {
                                    Some(r) if r < -80 => "信号弱，请靠近设备".to_string(),
                                    _ => String::new(),
                                };
                                let frame = HrFrame {
                                    bpm,
                                    status: HrStatus::Live,
                                    device: Some(DeviceInfo { name, battery: None, rssi }),
                                    message: msg,
                                    trigger: None,
                                    threshold: None,
                                    intensity: 0.0,
                                    external_events: Vec::new(),
                                };
                                *current.lock().await = frame.clone();
                                let _ = tx.send(frame);
                            }
                        }
                        Some(_) => continue,
                        None => break, // 设备断开
                    }
                }
                _ = matcher_rx.changed() => {
                    let _ = periph.disconnect().await;
                    continue 'outer;
                }
            }
        }

        // 设备断开：广播 device_lost 并回到外层重连（保持 BLE 模式）
        log::warn!("心率设备断开，尝试重连…");
        let lost = HrFrame {
            bpm: 0,
            status: HrStatus::DeviceLost,
            device: None,
            message: "设备已断开，重连中…".to_string(),
            trigger: None,
            threshold: None,
            intensity: 0.0,
            external_events: Vec::new(),
        };
        *current.lock().await = lost.clone();
        let _ = tx.send(lost);
        tokio::time::sleep(RECONNECT_BACKOFF).await;
    }
}

/// 设备是否广播心率服务 0x180D。
/// 仅匹配显式声明 HR Service 的设备，避免盲目连接手机/耳机等无关外设。
fn advertises_hr(props: &btleplug::api::PeripheralProperties) -> bool {
    props.services.contains(&HR_SERVICE)
}

/// 解析 HR Measurement (0x2A37)。
/// 首字节 flag：bit0 = 心率格式（0:uint8 / 1:uint16）；
/// bit1 = 能量消耗紧随其后；bit2 = RR 间隔紧随其后（均在心率值之后，不影响取值位置）。
fn parse_hr(value: &[u8]) -> Option<u32> {
    if value.is_empty() {
        return None;
    }
    let flags = value[0];
    if flags & 0x01 == 0 {
        if value.len() < 2 {
            return None;
        }
        Some(value[1] as u32)
    } else {
        if value.len() < 3 {
            return None;
        }
        let lo = value[1] as u32;
        let hi = value[2] as u32;
        Some(lo | (hi << 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uint8() {
        assert_eq!(parse_hr(&[0x00, 72]), Some(72));
    }
    #[test]
    fn parse_uint16() {
        // 0x03E8 = 1000 bpm
        assert_eq!(parse_hr(&[0x01, 0xE8, 0x03]), Some(1000));
    }
    #[test]
    fn parse_uint8_with_energy_and_rr() {
        // flag=0x06 (能量+RR)，心率 uint8=80，后续 energy(2)+rr(2) 不影响取值
        let v = [0x06, 80, 0x10, 0x00, 0x20, 0x00];
        assert_eq!(parse_hr(&v), Some(80));
    }
    #[test]
    fn parse_uint16_with_trailing_fields() {
        // flag=0x01(uint16) HR=0x005A=90，后面多字节忽略
        let v = [0x01, 0x5A, 0x00, 0xFF, 0xFF];
        assert_eq!(parse_hr(&v), Some(90));
    }
    #[test]
    fn parse_too_short() {
        assert_eq!(parse_hr(&[]), None);
        assert_eq!(parse_hr(&[0x01]), None); // uint16 但数据不足
    }
}
