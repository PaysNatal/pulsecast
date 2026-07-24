//! 怦然 PulseCast · 内嵌本地心率服务
//!
//! 完全等价于 `dev-server.mjs`：在同一端口（默认 4567）上同时提供
//! 1. HTTP 静态托管：首页(FTUE) `/`、设置 `/settings`、OBS 叠层 `/obs`、设计令牌 `/design-tokens.css`
//! 2. WebSocket `ws://localhost:4567`（根路径升级）按 1Hz 推送心率帧
//!
//! 这样 OBS 浏览器源与 Tauri 桌面窗口共用同一数据源，前端三页不再依赖 Node。
//! BLE 真实读取见 `ble` 模块（feature `ble`，沙箱/无蓝牙时自动回退模拟源）。

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Json, State, WebSocketUpgrade};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, watch, Mutex};
use tokio::task::JoinHandle;

// BLE 真实心率读取（仅在 --features ble 时编译，缺省走模拟源）
#[cfg(feature = "ble")]
pub mod ble;

// VRChat OSC 适配（心率 → avatar 参数 + ChatBox，零依赖 UDP）
pub mod osc;

// 设置持久化（JSON 配置文件读写）
pub mod config;

// OBS WebSocket v5 客户端（一键注入浏览器源）
pub mod obs_ws;

// 设备 Profile（小米/华为/Amazfit/Polar 适配引导）
pub mod device_profiles;

// 名场面系统（心率峰值自动标记 + OBS Replay Buffer 联动）
pub mod highlights;

// ── 数据模型（与 dev-server.mjs / obs-overlay.html 契约一致） ───────────────

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HrStatus {
    Connecting,
    Live,
    DeviceLost,
    Error,
}

#[derive(Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<i16>,
}

#[derive(Clone, Serialize)]
pub struct HrFrame {
    pub bpm: u32,
    pub status: HrStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<DeviceInfo>,
    #[serde(default)]
    pub message: String,
    /// 阈值越界动作（仅在状态切换的瞬间出现，平时为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<TriggerKind>,
    /// 阈值配置回声（OBS 叠层据此绘制参考线；未配置阈值为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<ThresholdCfg>,
    /// 心率强度 0.0~1.0（基于静息/最大心率归一化，驱动氛围引擎梯度效果）
    #[serde(default)]
    pub intensity: f32,
    /// 外部事件（直播间弹幕/礼物等，当前预留，v1.x 接入）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_events: Vec<ExternalEvent>,
}

/// 外部事件源（预留弹幕/礼物联动接口）
#[derive(Clone, Serialize)]
#[serde(tag = "source")]
pub enum ExternalEvent {
    /// VRChat OSC 参数
    Osc { param: String, value: f32 },
    /// 直播间事件（B 站弹幕 / 抖音礼物 / SC，v1.x 实现）
    LiveRoom {
        platform: String,
        kind: String,
        user: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
    },
}

/// 阈值配置：bpm 高于 high → High 动作；低于 low → Low 动作；介于之间 → Normal
#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq)]
pub struct ThresholdCfg {
    pub high: u32,
    pub low: u32,
}

/// 阈值越界动作类型（与 obs-overlay.html / obs-pulsecast 约定一致）
#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    High,
    Low,
    Normal,
}

/// 远程控制事件（移动端 → 服务端 → 广播给所有订阅方，含 OBS 叠层/原生模块）
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    SwitchScene,
    SetThreshold,
    Flash,
    Reset,
}

/// 控制事件载荷（与 obs-overlay.html / pulsecast-hr-source.c 约定一致）
///
/// 契约要点：序列化时额外带 `"type":"control"`，便于原生模块(OBS C 端)用
/// `pc_ws_parse_control_kind()` 在海量心率帧中秒级判别控制事件；前端浏览器
/// 叠层仍按 `kind` 区分动作（`applyState` 中 `if (msg.kind) ...`）。
#[derive(Clone, Serialize)]
pub struct ControlEvent {
    /// 固定为 "control"，原生模块据此识别控制事件（与前端按 kind 区分互补）
    #[serde(rename = "type")]
    pub event_type: String,
    pub kind: ControlKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u32>,
    /// 来源：mobile / api / obs
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub at: u64,
}

#[allow(dead_code)]
fn default_source() -> String {
    "server".to_string()
}

impl HrFrame {
    fn connecting() -> Self {
        HrFrame {
            bpm: 0,
            status: HrStatus::Connecting,
            device: None,
            message: "正在连接设备…".to_string(),
            trigger: None,
            threshold: None,
            intensity: 0.0,
            external_events: Vec::new(),
        }
    }
    fn live(bpm: u32, device: DeviceInfo) -> Self {
        HrFrame {
            bpm,
            status: HrStatus::Live,
            device: Some(device),
            message: String::new(),
            trigger: None,
            threshold: None,
            intensity: 0.0,
            external_events: Vec::new(),
        }
    }
}

// ── 阈值引擎 ────────────────────────────────────────────────────────────
// 内部分区：心率相对阈值落入 High / Low / Normal 三区；仅在跨越边界的瞬间
// 由 transform 任务注入 trigger 事件，避免每帧重复广播动作。

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Zone {
    High,
    Low,
    Normal,
}

/// 纯函数：根据阈值评估当前 bpm 所在分区（连接中 bpm=0 视为 Normal）
fn evaluate(bpm: u32, th: &ThresholdCfg) -> Zone {
    if bpm == 0 {
        return Zone::Normal;
    }
    if bpm > th.high {
        Zone::High
    } else if bpm < th.low {
        Zone::Low
    } else {
        Zone::Normal
    }
}

/// 给裸帧注入 threshold 配置回声 + intensity 梯度；若分区变化则注入 trigger。
async fn annotate(
    frame: HrFrame,
    threshold: &Option<ThresholdCfg>,
    last_zone: &Arc<Mutex<Option<Zone>>>,
) -> HrFrame {
    let mut frame = frame;

    // 心率强度归一化：(bpm - 60) / (200 - 60)，clamp [0, 1]
    // 60 = 默认静息心率，200 = 默认最大心率（后续可配置化）
    if frame.bpm > 0 {
        frame.intensity = ((frame.bpm as f32 - 60.0) / 140.0).clamp(0.0, 1.0);
    }

    if let Some(th) = threshold {
        frame.threshold = Some(*th);
        let zone = evaluate(frame.bpm, th);
        let mut last = last_zone.lock().await;
        let prev = *last;
        if prev != Some(zone) {
            if prev.is_some() {
                frame.trigger = Some(match zone {
                    Zone::High => TriggerKind::High,
                    Zone::Low => TriggerKind::Low,
                    Zone::Normal => TriggerKind::Normal,
                });
            }
            *last = Some(zone);
        }
    }
    frame
}

// ── 配置 ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Opts {
    /// 前端文件根目录（含 ftue.html / settings.html / obs-overlay.html / design-tokens.css）
    pub webroot: PathBuf,
    /// 监听端口
    pub port: u16,
    /// 强制模拟心率源（开发/无蓝牙环境）
    pub mock: bool,
    /// 优先尝试真实 BLE 读取（需 --features ble 编译）
    pub real: bool,
    /// 启用 VRChat OSC 转发（心率 → avatar 参数）
    pub vrchat: bool,
    /// VRChat OSC 监听地址（默认 127.0.0.1:9000）
    pub osc_addr: String,
    /// 同时把心率写入 VRChat ChatBox
    pub chatbox: bool,
    /// 心率阈值（high/low）；配置后服务端注入 trigger 与 threshold 广播
    pub threshold: Option<ThresholdCfg>,
    /// WS 连接认证 token（Tauri 启动时生成，前端通过 IPC 获取后附在 WS URL 参数中）
    pub token: Option<String>,
    /// 启动时自动连接的设备名（配置零感知：记住上次设备）
    pub initial_device: Option<String>,
}

// ── 共享状态 ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    webroot: PathBuf,
    tx: broadcast::Sender<HrFrame>,
    ctl_tx: broadcast::Sender<ControlEvent>,
    current: Arc<Mutex<HrFrame>>,
    /// 运行时阈值（可被移动端 set_threshold 命令实时修改）
    threshold: Arc<Mutex<Option<ThresholdCfg>>>,
    /// 热源控制器（模拟 / 真实 BLE，运行时可热切换）
    controller: SourceController,
    /// WS 认证 token（None 表示不校验，兼容独立运行模式）
    token: Option<String>,
    /// 名场面引擎（ring buffer + 峰值检测）
    highlights: Arc<Mutex<highlights::HighlightsEngine>>,
}

impl AppState {
    async fn current(&self) -> HrFrame {
        self.current.lock().await.clone()
    }
}

// ── 模拟心率源（随机游走，接口与真实源一致） ───────────────────────────────

struct Sim {
    bpm: u32,
    seed: u64,
}

impl Sim {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9abc_def0);
        Sim { bpm: 72, seed }
    }
    fn step(&mut self) -> u32 {
        // 线性同余发生器，避免引入额外依赖
        self.seed = self
            .seed
            .wrapping_mul(1103515245)
            .wrapping_add(12345);
        let r = ((self.seed >> 16) % 7) as i32; // 0..6
        let delta = r - 3; // -3..2
        let nb = (self.bpm as i32 + delta).clamp(52, 178);
        self.bpm = nb as u32;
        self.bpm
    }
}

/// 模拟热源：在 mode==Mock 期间每秒推送一帧；模式切走即退出（由 supervisor 回收）。
async fn mock_task(
    tx: broadcast::Sender<HrFrame>,
    current: Arc<Mutex<HrFrame>>,
    mut mode_rx: watch::Receiver<SourceMode>,
    my: SourceMode,
) {
    let connecting = HrFrame::connecting();
    let _ = tx.send(connecting.clone());
    *current.lock().await = connecting;
    // 模拟“连接中 → 实时”的过渡
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut sim = Sim::new();
    let device = DeviceInfo {
        name: "Amazfit GTR 4（模拟）".to_string(),
        battery: Some(88),
        rssi: Some(-55),
    };
    loop {
        if *mode_rx.borrow() != my {
            break;
        }
        let frame = HrFrame::live(sim.step(), device.clone());
        *current.lock().await = frame.clone();
        let _ = tx.send(frame);
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            _ = mode_rx.changed() => {
                if *mode_rx.borrow() != my {
                    break;
                }
            }
        }
    }
}

// ── 热源控制器（运行时切换 模拟 / 真实 BLE） ──────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SourceMode {
    Mock,
    Ble,
}

impl SourceMode {
    fn as_str(self) -> &'static str {
        match self {
            SourceMode::Mock => "mock",
            SourceMode::Ble => "ble",
        }
    }
}

#[derive(Clone)]
struct SourceController {
    inner_tx: broadcast::Sender<HrFrame>,
    src_current: Arc<Mutex<HrFrame>>,
    mode_tx: Arc<watch::Sender<SourceMode>>,
    #[cfg(feature = "ble")]
    matcher_tx: Arc<watch::Sender<Option<String>>>,
}

impl SourceController {
    /// 切换数据源（mock / ble）。切换时 supervisor 自动回收旧热源任务。
    fn set_mode(&self, m: SourceMode) {
        let _ = self.mode_tx.send(m);
    }
    /// 选择要连接的具体 BLE 设备（名称/id 子串）；None = 自动选第一台心率设备。
    #[cfg(feature = "ble")]
    fn set_device(&self, name: Option<String>) {
        let _ = self.matcher_tx.send(name);
    }
    fn mode(&self) -> SourceMode {
        *self.mode_tx.borrow()
    }
    /// 扫描附近心率设备（仅 ble feature 可用）
    #[cfg(feature = "ble")]
    async fn scan(&self) -> Vec<crate::ble::ScanDevice> {
        crate::ble::scan(Duration::from_secs(5)).await.unwrap_or_default()
    }

    /// 启动 supervisor：按当前 mode 运行对应热源；模式变化时热切换。
    fn run(self) {
        tokio::spawn(async move {
            let mut mode_rx = self.mode_tx.subscribe();
            let mut task: Option<JoinHandle<()>> = None;
            let mut active = *mode_rx.borrow();
            loop {
                if task.is_none() {
                    let inner = self.inner_tx.clone();
                    let cur = self.src_current.clone();
                    task = Some(match active {
                        SourceMode::Mock => {
                            let rx = self.mode_tx.subscribe();
                            tokio::spawn(async move {
                                mock_task(inner, cur, rx, SourceMode::Mock).await;
                            })
                        }
                        SourceMode::Ble => {
                            #[cfg(feature = "ble")]
                            {
                                let mrx = self.matcher_tx.subscribe();
                                tokio::spawn(async move {
                                    ble_task(inner, cur, mrx, SourceMode::Ble).await;
                                })
                            }
                            #[cfg(not(feature = "ble"))]
                            {
                                // 未编译 ble 时不应到达（initial 已强制 Mock），兜底跑模拟
                                let rx = self.mode_tx.subscribe();
                                tokio::spawn(async move {
                                    mock_task(inner, cur, rx, SourceMode::Mock).await;
                                })
                            }
                        }
                    });
                }
                if mode_rx.changed().await.is_err() {
                    break;
                }
                let new = *mode_rx.borrow();
                if new != active {
                    active = new;
                    if let Some(h) = task.take() {
                        h.abort();
                    }
                }
            }
        });
    }
}

/// BLE 热源：循环启动 run_hr；断线/异常自动重试；matcher 变更时 run_hr 自行重连。
#[cfg(feature = "ble")]
async fn ble_task(
    tx: broadcast::Sender<HrFrame>,
    current: Arc<Mutex<HrFrame>>,
    matcher_rx: watch::Receiver<Option<String>>,
    _my: SourceMode,
) {
    loop {
        match crate::ble::run_hr(tx.clone(), current.clone(), matcher_rx.clone()).await {
            Ok(()) => break, // run_hr 仅在 matcher 变更 / 致命错误时返回
            Err(e) => {
                log::warn!("BLE 热源异常，3s 后重试：{e}");
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

// ── Origin 校验中间件 ──────────────────────────────────────────────────────

fn is_allowed_origin(origin: &str) -> bool {
    // 精确匹配 localhost/127.0.0.1 前缀（防止 http://localhost.evil.com 绕过）
    if origin == "http://localhost"
        || origin.starts_with("http://localhost:")
        || origin == "http://127.0.0.1"
        || origin.starts_with("http://127.0.0.1:")
    {
        return true;
    }
    if origin == "tauri://localhost" || origin == "https://tauri.localhost" {
        return true;
    }
    false
}

async fn origin_guard(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    if let Some(origin) = req.headers().get(header::ORIGIN) {
        if let Ok(val) = origin.to_str() {
            if !is_allowed_origin(val) {
                return (StatusCode::FORBIDDEN, "Origin 不允许").into_response();
            }
        }
    }
    next.run(req).await
}

// ── API Token 校验中间件 ────────────────────────────────────────────────────
// 仅拦截 /api/* 路由；token 未配置时（独立运行模式）放行所有请求。
// 支持两种传递方式：?token=<value> 查询参数 或 Authorization: Bearer <value> 头。

async fn token_guard(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    // 仅守卫 /api/* 路由
    if !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }
    // 未配置 token → 兼容独立运行 / 开发模式，放行
    let expected = match &state.token {
        Some(t) => t,
        None => return next.run(req).await,
    };
    // 方式一：查询参数 ?token=xxx
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(val) = pair.strip_prefix("token=") {
                if val == expected {
                    return next.run(req).await;
                }
            }
        }
    }
    // 方式二：Authorization: Bearer xxx
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(val) = auth.to_str() {
            if let Some(tok) = val.strip_prefix("Bearer ") {
                if tok == expected {
                    return next.run(req).await;
                }
            }
        }
    }
    (StatusCode::UNAUTHORIZED, "未授权：缺少有效 token").into_response()
}

// ── 入口 ───────────────────────────────────────────────────────────────────

pub async fn run_server(opts: Opts) {
    // 双层通道：源 → inner_tx（裸帧）；transform → tx（注入阈值后对外广播）
    let (inner_tx, _inner_rx) = broadcast::channel::<HrFrame>(16);
    let (tx, _rx) = broadcast::channel::<HrFrame>(16);
    let (ctl_tx, _ctl_rx) = broadcast::channel::<ControlEvent>(16);
    let current = Arc::new(Mutex::new(HrFrame::connecting()));
    let src_current = Arc::new(Mutex::new(HrFrame::connecting()));
    // 阈值用共享 Arc<Mutex> 持有，支持移动端运行时修改
    let threshold = Arc::new(Mutex::new(opts.threshold));

    // 热源控制器：按初始模式（real → BLE，否则 → 模拟）启动，支持运行时热切换。
    let initial_mode = if opts.real && !opts.mock {
        #[cfg(feature = "ble")]
        {
            SourceMode::Ble
        }
        #[cfg(not(feature = "ble"))]
        {
            SourceMode::Mock
        }
    } else {
        SourceMode::Mock
    };
    let (mode_tx, _) = watch::channel(initial_mode);
    #[cfg(feature = "ble")]
    let (matcher_tx, _) = watch::channel::<Option<String>>(opts.initial_device.clone());
    let controller = SourceController {
        inner_tx: inner_tx.clone(),
        src_current: src_current.clone(),
        mode_tx: Arc::new(mode_tx),
        #[cfg(feature = "ble")]
        matcher_tx: Arc::new(matcher_tx),
    };
    let state = AppState {
        webroot: opts.webroot.clone(),
        tx: tx.clone(),
        ctl_tx: ctl_tx.clone(),
        current: current.clone(),
        threshold: threshold.clone(),
        controller: controller.clone(),
        token: opts.token.clone(),
        highlights: Arc::new(Mutex::new(highlights::HighlightsEngine::new())),
    };

    // 阈值转换任务：订阅裸帧 → 注入 threshold/trigger → 对外广播
    let last_zone = Arc::new(Mutex::new(None::<Zone>));
    {
        let mut inner_rx = inner_tx.subscribe();
        let out_tx = tx.clone();
        let out_current = current.clone();
        let lz = last_zone.clone();
        let thr = threshold.clone();
        let hl = state.highlights.clone();
        tokio::spawn(async move {
            let mut prev_hl_count: usize = 0;
            let mut last_replay_at: u64 = 0; // Replay Buffer 冷却（30s）
            while let Ok(f) = inner_rx.recv().await {
                let th = *thr.lock().await;
                let annotated = annotate(f, &th, &lz).await;
                // 喂给名场面引擎
                if annotated.bpm > 0 {
                    let mut engine = hl.lock().await;
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    engine.push(highlights::TimestampedFrame {
                        at: now_ms,
                        bpm: annotated.bpm,
                        intensity: annotated.intensity,
                    });
                    // 检测新名场面 → 触发 OBS Replay Buffer（30s 冷却）
                    if engine.last_event_is_new(prev_hl_count)
                        && now_ms.saturating_sub(last_replay_at) > 30_000
                    {
                        engine.mark_replay_saved();
                        last_replay_at = now_ms;
                        prev_hl_count = engine.events().len();
                        drop(engine);
                        tokio::spawn(async {
                            let cfg = config::load().await;
                            let addr = cfg.obs_ws_addr.as_deref().unwrap_or("ws://localhost:4455");
                            let pwd = cfg.obs_ws_password.as_deref();
                            if let Ok(mut client) = obs_ws::ObsWsClient::connect(addr, pwd).await {
                                let _ = client.trigger_replay_buffer().await;
                                log::info!("[pulsecast] 名场面触发 OBS Replay Buffer 保存");
                            }
                        });
                    } else {
                        prev_hl_count = engine.events().len();
                    }
                }
                *out_current.lock().await = annotated.clone();
                let _ = out_tx.send(annotated);
            }
        });
    }

    // 热源控制器：按 initial_mode 启动 supervisor，并支持运行时热切换（模拟 ↔ BLE）。
    controller.run();

    // VRChat OSC 转发（可选）
    if opts.vrchat {
        crate::osc::spawn(&tx, opts.osc_addr.clone(), opts.chatbox);
    }

    let app = {
        let mut r = Router::new()
            .route("/", get(root_handler))
            .route("/ws", get(ws_handler))
            .route("/settings", get(settings_handler))
            .route("/obs", get(obs_handler))
            .route("/design-tokens.css", get(css_handler))
            .route("/api/source", get(get_source).post(post_source))
            .route("/api/config", get(get_config).post(post_config))
            .route("/api/obs/status", get(obs_status_handler))
            .route("/api/obs/inject", post(obs_inject_handler))
            .route("/api/obs/remove", post(obs_remove_handler))
            .route("/api/highlights", get(highlights_handler))
            .route("/api/highlights/card/{index}", get(highlights_card_handler))
            .route("/api/ble/profiles", get(ble_profiles_handler));
        #[cfg(feature = "ble")]
        {
            r = r.route("/api/ble/scan", get(scan_handler));
        }
        r.layer(axum::middleware::from_fn_with_state(state.clone(), token_guard))
            .layer(axum::middleware::from_fn(origin_guard))
            .with_state(state.clone())
    };

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, opts.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            log::error!("无法绑定 {addr}: {e}");
            return;
        }
    };
    log::info!("怦然 PulseCast 服务已启动 → http://localhost:{}", opts.port);
    if let Err(e) = axum::serve(listener, app).await {
        log::error!("服务异常退出: {e}");
    }
}

// ── HTTP 处理 ──────────────────────────────────────────────────────────────

async fn root_handler(State(state): State<AppState>) -> Response {
    serve_file(&state, "ftue.html").await
}

/// WebSocket 升级端点（axum 0.8 起 WebSocketUpgrade 为 FromRequestParts，
/// 与静态首页分离到独立路径，避免 Option 包裹的兼容问题）。
/// 若服务端配置了 token，则要求 WS URL 携带 ?token=<value>，否则拒绝升级。
async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Some(expected) = &state.token {
        match params.get("token") {
            Some(t) if t == expected => {}
            _ => {
                return (StatusCode::FORBIDDEN, "未授权连接").into_response();
            }
        }
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

async fn settings_handler(State(state): State<AppState>) -> Response {
    serve_file(&state, "settings.html").await
}

async fn obs_handler(State(state): State<AppState>) -> Response {
    serve_file(&state, "obs-overlay.html").await
}

async fn css_handler(State(state): State<AppState>) -> Response {
    serve_file(&state, "design-tokens.css").await
}

// ── 热源控制 API（桌面设置页 / 移动端驱动） ────────────────────────────────

#[derive(Deserialize)]
struct SourceReq {
    mode: String,
    #[serde(default)]
    device: Option<String>,
}

/// GET /api/source → 当前数据源模式（"mock" | "ble"）
async fn get_source(State(state): State<AppState>) -> Response {
    let mode = state.controller.mode();
    Json(json!({ "mode": mode.as_str() })).into_response()
}

/// POST /api/source {"mode":"ble"|"mock","device":可选名称} → 运行时热切换热源
async fn post_source(State(state): State<AppState>, Json(body): Json<SourceReq>) -> Response {
    match body.mode.as_str() {
        "ble" => {
            #[cfg(feature = "ble")]
            {
                state.controller.set_device(body.device);
            }
            state.controller.set_mode(SourceMode::Ble);
        }
        "mock" => {
            state.controller.set_mode(SourceMode::Mock);
        }
        other => {
            return (StatusCode::BAD_REQUEST, format!("未知数据源：{other}")).into_response();
        }
    }
    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

/// GET /api/ble/scan → 扫描附近心率设备（仅 ble feature 可用）
#[cfg(feature = "ble")]
async fn scan_handler(State(state): State<AppState>) -> Response {
    let devs = state.controller.scan().await;
    Json(json!({ "devices": devs })).into_response()
}

/// GET /api/config → 读取持久化配置
async fn get_config() -> Response {
    let cfg = config::load().await;
    Json(cfg).into_response()
}

/// POST /api/config → 保存配置到磁盘
async fn post_config(Json(body): Json<config::AppConfig>) -> Response {
    match config::save(&body).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

// ── OBS WebSocket API ─────────────────────────────────────────────────────

async fn obs_status_handler(State(_state): State<AppState>) -> Response {
    let cfg = config::load().await;
    let addr = cfg.obs_ws_addr.as_deref().unwrap_or("ws://localhost:4455");
    let pwd = cfg.obs_ws_password.as_deref();
    let status = obs_ws::status(addr, pwd).await;
    Json(status).into_response()
}

async fn obs_inject_handler(State(state): State<AppState>) -> Response {
    let cfg = config::load().await;
    let addr = cfg.obs_ws_addr.as_deref().unwrap_or("ws://localhost:4455");
    let pwd = cfg.obs_ws_password.as_deref();
    let token_part = state
        .token
        .as_ref()
        .map(|t| format!("&token={t}"))
        .unwrap_or_default();
    let url = format!(
        "http://localhost:{}/obs?style=pill{}",
        cfg.port.unwrap_or(4567),
        token_part
    );
    match obs_ws::inject(addr, pwd, &url).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "ok": false, "message": format!("{e:#}") })),
        )
            .into_response(),
    }
}

async fn obs_remove_handler(State(_state): State<AppState>) -> Response {
    let cfg = config::load().await;
    let addr = cfg.obs_ws_addr.as_deref().unwrap_or("ws://localhost:4455");
    let pwd = cfg.obs_ws_password.as_deref();
    match obs_ws::ObsWsClient::connect(addr, pwd).await {
        Ok(mut client) => match client.remove_input().await {
            Ok(()) => Json(json!({ "ok": true, "message": "已删除 OBS 心率源" })).into_response(),
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "ok": false, "message": format!("{e:#}") })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "ok": false, "message": format!("连接 OBS 失败: {e:#}") })),
        )
            .into_response(),
    }
}

// ── 名场面 API ────────────────────────────────────────────────────────────

async fn highlights_handler(State(state): State<AppState>) -> Response {
    let engine = state.highlights.lock().await;
    Json(engine.events()).into_response()
}

async fn highlights_card_handler(
    State(state): State<AppState>,
    axum::extract::Path(index): axum::extract::Path<usize>,
) -> Response {
    let engine = state.highlights.lock().await;
    match engine.export_card_svg(index) {
        Some(svg) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            svg,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "名场面不存在或数据不足").into_response(),
    }
}

// ── 设备 Profile API ──────────────────────────────────────────────────────

async fn ble_profiles_handler() -> Response {
    Json(device_profiles::all_profiles()).into_response()
}

async fn serve_file(state: &AppState, name: &str) -> Response {
    match tokio::fs::read(state.webroot.join(name)).await {
        Ok(bytes) => {
            let ct = if name.ends_with(".css") {
                "text/css; charset=utf-8"
            } else {
                "text/html; charset=utf-8"
            };
            let mut resp = Response::new(bytes.into());
            if let Ok(v) = HeaderValue::from_str(ct) {
                resp.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            resp
        }
        Err(_) => (StatusCode::NOT_FOUND, "未找到该资源").into_response(),
    }
}

// ── WebSocket 处理 ─────────────────────────────────────────────────────────

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: AppState) {
    use axum::extract::ws::Message;

    let mut rx = state.tx.subscribe();
    let mut ctl_rx = state.ctl_tx.subscribe();
    // 新连接立即推送当前帧，避免 OBS 端空白等待
    if let Ok(initial) = serde_json::to_string(&state.current().await) {
        let _ = socket.send(Message::Text(initial.into())).await;
    }

    loop {
        tokio::select! {
            frame = rx.recv() => {
                match frame {
                    Ok(f) => {
                        if let Ok(text) = serde_json::to_string(&f) {
                            if socket.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break, // 发送端已关闭
                }
            }
            ctl = ctl_rx.recv() => {
                if let Ok(ev) = ctl {
                    if let Ok(text) = serde_json::to_string(&ev) {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break, // 连接错误，退出
                    // 客户端文本：解析为远程控制命令（移动端 → 服务端）
                    Some(Ok(Message::Text(t))) => {
                        if let Some(ev) = handle_command(&t, &state.threshold).await {
                            let _ = state.ctl_tx.send(ev);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

/// 解析客户端发来的控制命令；非法或缺失必要字段则返回 None（静默忽略）。
/// 合法命令会回写为 ControlEvent（source=mobile）并广播给所有订阅方；
/// 其中 set_threshold 还会实时更新运行时阈值。
async fn handle_command(raw: &str, thr: &Arc<Mutex<Option<ThresholdCfg>>>) -> Option<ControlEvent> {
    let cmd: IncomingCommand = serde_json::from_str(raw).ok()?;
    let kind = match cmd.kind.as_str() {
        "switch_scene" => ControlKind::SwitchScene,
        "set_threshold" => ControlKind::SetThreshold,
        "flash" => ControlKind::Flash,
        "reset" => ControlKind::Reset,
        _ => return None,
    };
    match kind {
        ControlKind::SwitchScene => {
            let scene = cmd.scene?;
            if scene.trim().is_empty() {
                return None;
            }
            Some(ControlEvent {
                event_type: "control".to_string(),
                kind,
                scene: Some(scene),
                high: None,
                low: None,
                source: "mobile".to_string(),
                at: now_ms(),
            })
        }
        ControlKind::SetThreshold => {
            let high = cmd.high?;
            let low = cmd.low?;
            if low >= high {
                return None;
            }
            // 运行时更新阈值（异步锁，避免阻塞事件循环）
            *thr.lock().await = Some(ThresholdCfg { high, low });
            Some(ControlEvent {
                event_type: "control".to_string(),
                kind,
                scene: None,
                high: Some(high),
                low: Some(low),
                source: "mobile".to_string(),
                at: now_ms(),
            })
        }
        _ => Some(ControlEvent {
            event_type: "control".to_string(),
            kind,
            scene: None,
            high: None,
            low: None,
            source: "mobile".to_string(),
            at: now_ms(),
        }),
    }
}

/// 移动端发来的原始命令（仅用于反序列化，不对外广播）
#[derive(Deserialize)]
struct IncomingCommand {
    kind: String,
    #[serde(default)]
    scene: Option<String>,
    #[serde(default)]
    high: Option<u32>,
    #[serde(default)]
    low: Option<u32>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_zones_basic() {
        let th = ThresholdCfg { high: 150, low: 60 };
        assert_eq!(evaluate(0, &th), Zone::Normal); // 连接中
        assert_eq!(evaluate(60, &th), Zone::Normal); // 下沿边界
        assert_eq!(evaluate(59, &th), Zone::Low);
        assert_eq!(evaluate(72, &th), Zone::Normal);
        assert_eq!(evaluate(150, &th), Zone::Normal); // 上沿边界
        assert_eq!(evaluate(151, &th), Zone::High);
    }

    #[test]
    fn evaluate_high_only_threshold() {
        let th = ThresholdCfg { high: 120, low: 1 };
        assert_eq!(evaluate(40, &th), Zone::Normal);
        assert_eq!(evaluate(121, &th), Zone::High);
    }

    #[tokio::test]
    async fn annotate_injects_trigger_on_crossing() {
        let th = ThresholdCfg { high: 150, low: 60 };
        let lz = Arc::new(Mutex::new(None::<Zone>));
        let bare = |bpm| HrFrame {
            bpm,
            status: HrStatus::Live,
            device: None,
            message: String::new(),
            trigger: None,
            threshold: None,
            intensity: 0.0,
            external_events: Vec::new(),
        };
        // 72 → Normal，无 trigger，但带 threshold 回声
        let f1 = annotate(bare(72), &Some(th), &lz).await;
        assert_eq!(f1.trigger, None);
        assert_eq!(f1.threshold, Some(th));
        // 跨到 160 → High，注入 trigger=High
        let f2 = annotate(bare(160), &Some(th), &lz).await;
        assert_eq!(f2.trigger, Some(TriggerKind::High));
        // 仍高区 165 → 未跨边界，无新 trigger
        let f3 = annotate(bare(165), &Some(th), &lz).await;
        assert_eq!(f3.trigger, None);
    }

    #[tokio::test]
    async fn handle_command_validates_and_builds_event() {
        let thr = std::sync::Arc::new(tokio::sync::Mutex::new(None::<ThresholdCfg>));
        // 非法 kind → None
        assert!(handle_command("{\"kind\":\"boom\"}", &thr).await.is_none());
        // switch_scene 缺 scene → None
        assert!(handle_command("{\"kind\":\"switch_scene\"}", &thr).await.is_none());
        // switch_scene 合法
        let e = handle_command("{\"kind\":\"switch_scene\",\"scene\":\"亲密时刻\"}", &thr).await.unwrap();
        assert_eq!(e.source, "mobile");
        assert!(matches!(e.kind, ControlKind::SwitchScene));
        assert_eq!(e.scene.as_deref(), Some("亲密时刻"));
        // set_threshold 非法（low>=high）→ None
        assert!(handle_command("{\"kind\":\"set_threshold\",\"high\":50,\"low\":80}", &thr).await.is_none());
        // set_threshold 合法 → 更新运行时阈值
        let e2 = handle_command("{\"kind\":\"set_threshold\",\"high\":140,\"low\":58}", &thr).await.unwrap();
        assert_eq!(e2.high, Some(140));
        assert_eq!(*thr.lock().await, Some(ThresholdCfg { high: 140, low: 58 }));
        // flash 合法
        let e3 = handle_command("{\"kind\":\"flash\"}", &thr).await.unwrap();
        assert!(matches!(e3.kind, ControlKind::Flash));
    }
}
