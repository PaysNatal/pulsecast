//! OBS WebSocket v5 客户端 —— 一键注入 / 管理 OBS 浏览器源
//!
//! 协议参考：https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md
//! OBS 28+ 内置 WebSocket 服务器，默认监听 ws://localhost:4455。
//!
//! 连接流程：
//!   1. TCP → ws://localhost:4455
//!   2. 收到 Hello (op:0)，含 authentication.challenge + salt（若设了密码）
//!   3. 计算认证：secret = b64(SHA256(pwd+salt))，auth = b64(SHA256(secret+challenge))
//!   4. 发送 Identify (op:1)
//!   5. 收到 Identified (op:2) → 握手完成
//!
//! 本模块为"连一次、做一件事、断开"的短连接模型，不维持长连接。

use anyhow::{bail, Context, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;
use serde_json::{json, Value};

type WsStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// OBS WebSocket 短连接客户端
pub struct ObsWsClient {
    stream: WsStream,
    req_id: u64,
}

/// OBS 连接状态（供 /api/obs/status 返回）
#[derive(serde::Serialize)]
pub struct ObsStatus {
    pub connected: bool,
    pub version: String,
    pub scene: String,
    pub has_source: bool,
}

/// 注入结果
#[derive(serde::Serialize)]
pub struct InjectResult {
    pub ok: bool,
    pub message: String,
    pub source_name: String,
}

const SOURCE_NAME: &str = "怦然心率";

impl ObsWsClient {
    /// 连接并完成认证握手
    pub async fn connect(addr: &str, password: Option<&str>) -> Result<Self> {
        let (mut stream, _) = tokio_tungstenite::connect_async(addr)
            .await
            .with_context(|| format!("连接 OBS WebSocket 失败 ({addr})"))?;

        // ── 1. 接收 Hello (op: 0) ──
        let hello = Self::recv_op(&mut stream, 0).await?;
        let d = &hello["d"];

        // ── 2. 认证 ──
        let auth_response = if let Some(auth) = d.get("authentication") {
            let challenge = auth["challenge"]
                .as_str()
                .context("Hello 缺少 authentication.challenge")?;
            let salt = auth["salt"]
                .as_str()
                .context("Hello 缺少 authentication.salt")?;
            let pwd = password.context("OBS 设置了密码，请在设置页填写 OBS WebSocket 密码")?;

            let secret = {
                let mut h = Sha256::new();
                h.update(format!("{pwd}{salt}").as_bytes());
                base64::engine::general_purpose::STANDARD.encode(h.finalize())
            };
            let auth = {
                let mut h = Sha256::new();
                h.update(format!("{secret}{challenge}").as_bytes());
                base64::engine::general_purpose::STANDARD.encode(h.finalize())
            };
            Some(auth)
        } else {
            None
        };

        // ── 3. 发送 Identify (op: 1) ──
        let identify = json!({
            "op": 1,
            "d": {
                "rpcVersion": 1,
                "authentication": auth_response,
            }
        });
        stream
            .send(Message::Text(identify.to_string().into()))
            .await
            .context("发送 Identify 失败")?;

        // ── 4. 接收 Identified (op: 2) ──
        Self::recv_op(&mut stream, 2).await?;

        Ok(Self { stream, req_id: 0 })
    }

    /// 发送 RPC 请求 (op:6) 并等待匹配的响应 (op:7)，10 秒超时
    async fn request(&mut self, request_type: &str, data: Value) -> Result<Value> {
        self.req_id += 1;
        let id = format!("pc-{}", self.req_id);

        let msg = json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": id,
                "requestData": data,
            }
        });
        self.stream
            .send(Message::Text(msg.to_string().into()))
            .await
            .with_context(|| format!("发送 {request_type} 请求失败"))?;

        // 等待匹配的 op:7 响应（跳过其他事件），10 秒超时
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            async {
                loop {
                    let resp = self.recv_op_raw().await?;
                    if resp["op"].as_u64() != Some(7) {
                        continue;
                    }
                    let d = &resp["d"];
                    if d["requestId"].as_str() != Some(&id) {
                        continue;
                    }
                    if d["requestStatus"]["result"].as_bool() != Some(true) {
                        let code = d["requestStatus"]["code"].as_u64().unwrap_or(0);
                        let comment = d["requestStatus"]["comment"]
                            .as_str()
                            .unwrap_or("未知错误");
                        bail!("OBS {request_type} 失败 (code {code}): {comment}");
                    }
                    return Ok(d["responseData"].clone());
                }
            },
        )
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => bail!("OBS {request_type} 响应超时 (10s)"),
        }
    }

    /// 获取 OBS 版本
    pub async fn get_version(&mut self) -> Result<String> {
        let resp = self.request("GetVersion", json!({})).await?;
        Ok(resp["obsVersion"].as_str().unwrap_or("unknown").to_string())
    }

    /// 获取当前节目场景名
    pub async fn get_current_scene(&mut self) -> Result<String> {
        let resp = self.request("GetCurrentProgramScene", json!({})).await?;
        Ok(resp["sceneName"]
            .as_str()
            .unwrap_or("Scene")
            .to_string())
    }

    /// 检查指定名称的输入源是否存在
    pub async fn find_input(&mut self, name: &str) -> Result<bool> {
        let resp = self
            .request("GetInputList", json!({ "inputKind": "browser_source" }))
            .await?;
        if let Some(inputs) = resp["inputs"].as_array() {
            Ok(inputs.iter().any(|i| i["inputName"].as_str() == Some(name)))
        } else {
            Ok(false)
        }
    }

    /// 创建浏览器源（怦然心率叠加层）
    pub async fn create_browser_source(
        &mut self,
        scene: &str,
        url: &str,
    ) -> Result<()> {
        self.request(
            "CreateInput",
            json!({
                "sceneName": scene,
                "inputName": SOURCE_NAME,
                "inputKind": "browser_source",
                "inputSettings": {
                    "url": url,
                    "width": 800,
                    "height": 200,
                    "css": "body { background: transparent; margin: 0; }",
                    "shutdown": true,
                },
            }),
        )
        .await?;
        Ok(())
    }

    /// 更新已有浏览器源的 URL（token 变化时调用）
    pub async fn update_input_url(&mut self, url: &str) -> Result<()> {
        self.request(
            "SetInputSettings",
            json!({
                "inputName": SOURCE_NAME,
                "inputSettings": { "url": url },
            }),
        )
        .await?;
        Ok(())
    }

    /// 删除怦然心率源
    pub async fn remove_input(&mut self) -> Result<()> {
        self.request("RemoveInput", json!({ "inputName": SOURCE_NAME }))
            .await?;
        Ok(())
    }

    /// 触发 OBS Replay Buffer 保存（名场面系统用）
    pub async fn trigger_replay_buffer(&mut self) -> Result<()> {
        self.request("TriggerReplayBuffer", json!({})).await?;
        Ok(())
    }

    // ── 内部辅助 ──

    /// 接收指定 op 的消息（10 秒超时）
    async fn recv_op(stream: &mut WsStream, expected_op: u64) -> Result<Value> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            async {
                loop {
                    let msg = stream
                        .next()
                        .await
                        .context("OBS WebSocket 连接已关闭")?
                        .context("读取 OBS WebSocket 消息失败")?;
                    match msg {
                        Message::Text(t) => {
                            let v: Value =
                                serde_json::from_str(&t).context("OBS 消息 JSON 解析失败")?;
                            if v["op"].as_u64() == Some(expected_op) {
                                return Ok(v);
                            }
                        }
                        Message::Close(_) => bail!("OBS WebSocket 连接被服务端关闭"),
                        _ => continue,
                    }
                }
            },
        )
        .await;
        match result {
            Ok(inner) => inner,
            Err(_) => bail!("OBS WebSocket 握手超时 (10s)"),
        }
    }

    /// 接收任意消息（用于 request 中跳过非响应消息）
    async fn recv_op_raw(&mut self) -> Result<Value> {
        loop {
            let msg = self
                .stream
                .next()
                .await
                .context("OBS WebSocket 连接已关闭")?
                .context("读取 OBS WebSocket 消息失败")?;
            match msg {
                Message::Text(t) => {
                    return serde_json::from_str(&t).context("OBS 消息 JSON 解析失败");
                }
                Message::Close(_) => bail!("OBS WebSocket 连接被服务端关闭"),
                _ => continue,
            }
        }
    }
}

/// 一键注入：连接 OBS → 创建/更新浏览器源
pub async fn inject(
    addr: &str,
    password: Option<&str>,
    overlay_url: &str,
) -> Result<InjectResult> {
    let mut client = ObsWsClient::connect(addr, password).await?;
    let scene = client.get_current_scene().await?;
    let exists = client.find_input(SOURCE_NAME).await.unwrap_or(false);

    if exists {
        client.update_input_url(overlay_url).await?;
        Ok(InjectResult {
            ok: true,
            message: format!("已更新「{SOURCE_NAME}」源地址（场景：{scene}）"),
            source_name: SOURCE_NAME.to_string(),
        })
    } else {
        client.create_browser_source(&scene, overlay_url).await?;
        Ok(InjectResult {
            ok: true,
            message: format!("已在场景「{scene}」中创建「{SOURCE_NAME}」浏览器源"),
            source_name: SOURCE_NAME.to_string(),
        })
    }
}

/// 获取 OBS 状态
pub async fn status(addr: &str, password: Option<&str>) -> ObsStatus {
    match ObsWsClient::connect(addr, password).await {
        Ok(mut client) => {
            let version = client.get_version().await.unwrap_or_default();
            let scene = client.get_current_scene().await.unwrap_or_default();
            let has_source = client.find_input(SOURCE_NAME).await.unwrap_or(false);
            ObsStatus {
                connected: true,
                version,
                scene,
                has_source,
            }
        }
        Err(_) => ObsStatus {
            connected: false,
            version: String::new(),
            scene: String::new(),
            has_source: false,
        },
    }
}
