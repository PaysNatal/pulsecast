//! 设置持久化 —— JSON 配置文件读写
//!
//! 配置存储在用户数据目录下 `pulsecast/config.json`，
//! 重启后保留设备配对、阈值、OBS 配置等用户设置。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub source_mode: Option<String>,
    #[serde(default)]
    pub hr_high: Option<u32>,
    #[serde(default)]
    pub hr_low: Option<u32>,
    #[serde(default)]
    pub obs_style: Option<String>,
    #[serde(default)]
    pub obs_scale: Option<f64>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub vrchat: Option<bool>,
    #[serde(default)]
    pub chatbox: Option<bool>,
    /// OBS WebSocket 地址（默认 ws://localhost:4455）
    #[serde(default)]
    pub obs_ws_addr: Option<String>,
    /// OBS WebSocket 密码（OBS → 工具 → WebSocket 服务器设置）
    #[serde(default)]
    pub obs_ws_password: Option<String>,
    /// 直播间地址（预留，v1.x 弹幕/礼物联动用）
    #[serde(default)]
    pub live_room_url: Option<String>,
}

pub fn config_path() -> PathBuf {
    let base = dirs_path();
    base.join("config.json")
}

fn dirs_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Library/Application Support/com.pulsecast.app")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("PulseCast")
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/pulsecast")
    }
}

pub fn load() -> AppConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
