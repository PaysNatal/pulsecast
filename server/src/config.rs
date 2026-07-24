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
    /// VRChat OSC 地址（默认 127.0.0.1:9000）
    #[serde(default)]
    pub osc_addr: Option<String>,
}

pub fn config_path() -> PathBuf {
    let base = dirs_path();
    base.join("config.json")
}

fn dirs_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| {
            log::warn!("HOME 环境变量未设置，配置将存储在当前目录");
            ".".to_string()
        });
        PathBuf::from(home).join("Library/Application Support/com.pulsecast.app")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
            log::warn!("APPDATA 环境变量未设置，配置将存储在当前目录");
            ".".to_string()
        });
        PathBuf::from(appdata).join("PulseCast")
    }
    #[cfg(target_os = "linux")]
    {
        // 优先使用 XDG_CONFIG_HOME，回退到 ~/.config
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("pulsecast");
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| {
            log::warn!("HOME 环境变量未设置，配置将存储在当前目录");
            ".".to_string()
        });
        PathBuf::from(home).join(".config/pulsecast")
    }
}

pub async fn load() -> AppConfig {
    let path = config_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("配置文件解析失败，使用默认值: {e}");
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

pub async fn save(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // 原子写入：先写临时文件再 rename，防止并发写入导致 JSON 损坏
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, &json)
        .await
        .map_err(|e| e.to_string())?;
    // Unix: 设置文件权限 0o600（仅所有者可读写，保护 obs_ws_password）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).await;
    }
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| e.to_string())
}
