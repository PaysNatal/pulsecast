# 怦然 PulseCast · 心率直播伴侣

开源、本地优先的心率直播叠加工具，为主播提供最便捷的服务：把 Zepp / Apple Watch / 胸带等心率设备，
通过**本地 BLE 直连 + 桌面悬浮 + OBS 浏览器源**，实时呈现在直播画面里。

- 本地 BLE 直连优先（标准心率服务 `0x180D` / 特征 `0x2A37`），不依赖海外云
- OBS 浏览器源三套样式（心跳药丸 / ECG 波形 / 极简大数字），透明背景、连接状态色
- 桌面设置 / 首次启动向导 / 托盘常驻
- 跨平台（Windows / macOS / Linux，Tauri v2 基座）

## 架构

```
心率设备 (BLE)
      │  标准 HR Profile 0x180D / 0x2A37
      ▼
Rust 后端  (btleplug 真实读取 / 无蓝牙时 mock 回退)
      │
      ▼
内嵌 HTTP + WebSocket 服务  :4567
      ├───────────────┼───────────────┐
 Tauri 桌面窗口     OBS 浏览器源     浏览器预览
 (FTUE / 设置)    (obs-overlay)    (调试)
```

**数据契约**（WebSocket，JSON，约 1Hz）：

```json
{
  "bpm": 71,
  "status": "live",
  "device": { "name": "Amazfit GTR 4", "battery": 88, "rssi": -55 },
  "message": ""
}
```

`status` ∈ `connecting` | `live` | `device_lost` | `error`（对应设计稿四态连接色）。

启用阈值（服务端 `--hr-high` / `--hr-low`）后，每帧额外携带：
- `threshold`: `{ "high": <u32>, "low": <u32> }`（OBS 叠层据此绘制参考线）
- `trigger`: 仅在跨区瞬间出现，取 `"high"` / `"low"` / `"normal"` 之一（`normal` 表示恢复），
  OBS 叠层据此触发边缘告警脉冲，`obs-pulsecast` 原生模块据此切换场景。

## 目录结构

| 路径 | 说明 |
|------|------|
| `ftue.html` | 首次启动向导（三步：选设备 → 一键连接 → 预览开播） |
| `settings.html` | 桌面设置窗口（设备连接 / 外观 / 叠层 / 端口 / 通用 / 热键） |
| `obs-overlay.html` | 自包含 OBS 浏览器源叠层（透明背景、3 样式、连接状态色） |
| `design-tokens.css` | 设计系统：CSS 变量 + 通用组件类（与叠层同源视觉） |
| `dev-server.mjs` | Node 版本地服务（开发参考实现） |
| `server/` | **Rust 内嵌服务**（axum + tokio），替代 `dev-server.mjs`；含 `osc.rs` VRChat OSC 转发 |
| `src-tauri/` | Tauri v2 桌面应用骨架 |
| `obs-pulsecast/` | **OBS 原生模块**（C/libobs）：心率源 + 心跳抖动/发光滤镜 + 阈值场景切换 |
| `docs/camera-and-vrchat.md` | 摄像头联动 & VRChat OSC 使用指南 |
| `mobile/` | **移动端伴侣 App 前端**（Tauri v2 iOS/Android）：`index.html` 三屏 SPA + `mobile.css` + `mobile.js`，复用 `design-tokens.css` |
| `tauri.mobile.conf.json` | Tauri v2 移动构建配置（iOS/Android + BLE 插件 + 权限） |
| `dist/` | 桌面打包用前端产物（`build-frontend.mjs` 生成） |
| `build-frontend.mjs` | 构建前把前端复制到 `dist/` |

## 运行方式

### A. Node 开发服务（最快预览）

```bash
NODE_PATH=~/.workbuddy/binaries/node/workspace/node_modules node dev-server.mjs
# 浏览器开 http://localhost:4567/ 看 FTUE；/settings 看设置
# OBS 浏览器源填 http://localhost:4567/obs
```

### B. Rust 内嵌服务（不再依赖 Node）

```bash
cd server
cargo run -- --webroot ..                 # 默认 mock 心率源
cargo run -- --webroot .. --mock          # 强制模拟
cargo run --features ble -- --webroot .. --real   # 启用真实 BLE 读取
cargo run -- --webroot .. --hr-high 150 --hr-low 55   # 启用阈值动作（越阈触发）
```

### C. Tauri 桌面应用

```bash
npm install
npm run tauri dev     # 开发：beforeDevCommand 起 Node 服务
npm run tauri build   # 生产：内嵌 Rust 服务给 OBS，前端打包进 dist
```

### D. 移动端伴侣 App（Tauri v2 iOS / Android）

手机与运行「怦然」桌面端的电脑处于同一局域网，App 通过 WebSocket 直连桌面服务拉取实时心率，
并用 Web Bluetooth（或 iOS/Android 原生 BLE 插件）扫描/连接心率设备。

```bash
# 1) 桌面端先运行 Rust 服务（手机填该电脑局域网 IP）
cargo run -p pulsecast-server -- --webroot .

# 2) 本地预览移动端（端口 4568）
npm run serve:mobile

# 3) 构建移动端安装包（需本机安装 Xcode / Android SDK，并在 src-tauri 配好 Tauri 移动环境）
npm run tauri:ios       # => IPA
npm run tauri:android   # => APK/AAB
```

> 移动端 `mobile/` 是纯静态 SPA，**不自带** Rust 服务：心率数据来自同局域网的桌面服务
> （在 App 右上角 ⚙ 里填 `服务器地址` + `端口`，默认 `localhost:4567`，仅当手机与桌面同机调试时生效）。
> iOS Safari 不支持 Web Bluetooth，国内安卓 WebView 支持有限，生产建议接入
> `@tauri-apps/plugin-ble` 原生蓝牙插件（配置见 `tauri.mobile.conf.json`）。

## OBS 配置

添加「浏览器源」→ URL 填 `http://localhost:4567/obs`，可选参数：

- `?style=pill|ecg|number` 切换样式
- `?scale=1.2` 缩放
- `?ws=ws://host:port` 自定义数据源地址
- `?demo=1` 强制演示模式（无连接也跳动）

**阈值动作**：浏览器源本身随 `trigger` 字段做边缘告警脉冲（high 红 / low 橙），并显示参考阈值；
真正的"场景切换"由 `obs-pulsecast` 原生模块在进程内完成（详见其 README）。阈值需在服务端用
`--hr-high` / `--hr-low` 开启。

## 延展能力

### OBS 原生模块（`obs-pulsecast/`）
比浏览器源性能更好、可与场景滤镜链深度组合：心率文字源 + 心跳镜头抖动 + 心跳发光。
需 OBS 开发包构建，详见 `obs-pulsecast/README.md`。

### 摄像头联动 & VRChat（`docs/camera-and-vrchat.md`）
- **摄像头联动**：把心跳抖动/发光滤镜挂到 OBS 摄像头源，画面随心跳反应（无需 rPPG）。
- **VRChat OSC**：`--vrchat` 启用，向 `127.0.0.1:9000` 发送 avatar 参数
  （`HeartRate` / `HeartRateFloat` / `HeartRatePercent` / `isHRConnected` / `ones|tens|hundredsHR`）
  与 ChatBox（`--chatbox`）。

```bash
cargo run -p pulsecast-server -- --webroot . --vrchat --chatbox
```

### 阈值动作（bpm 越阈触发）
心率越界时自动告警 / 切换场景，让直播在情绪高点自动"上特效"：
- **服务端检测**：`--hr-high` / `--hr-low` 配置阈值，跨区瞬间经 WebSocket 广播 `trigger` 事件。
- **OBS 浏览器源**：`obs-overlay.html` 接收 `trigger` → 整块源边缘脉冲（high 红 / low 橙）+ 显示参考阈值。
- **OBS 原生模块**：`obs-pulsecast` 的"心率源"越阈时在 OBS 进程内直接调用
  `obs_frontend_set_current_preview_scene2` 切换场景（零外部依赖），场景名在源属性里配置。

## 路线图（后续）

- 移动端已落地基础版（Tauri v2 mobile SPA + 构建配置），后续可加：原生 BLE 插件接入、心率源切换桌面场景、离线演示模式。

## 许可

MIT
