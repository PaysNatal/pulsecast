# 摄像头联动 & VRChat 适配

本文档说明怦然 PulseCast 两块「延展能力」的用法。两者都复用同一个本地心率服务
（`ws://localhost:4567`，由桌面端或 `cargo run -p pulsecast-server` 提供），无需额外配置设备。

---

## 一、摄像头联动（OBS）

「摄像头联动」= 让主播的**摄像头画面随心跳产生视觉反应**（抖动 / 泛光），
与叠层数字同步。技术上不需要 rPPG，直接把 `obs-pulsecast` 的心跳滤镜挂到摄像头源即可。

### 步骤
1. 启动怦然桌面端（或 `cargo run -p pulsecast-server -- --webroot .`）。
2. 在 OBS 里，右键你的**摄像头来源（视频采集设备）→ 滤镜**。
3. 添加滤镜：
   - **心跳镜头抖动 (PulseCast)**：每次心跳画面轻震，`抖动幅度` 建议 4–10px。
   - **心跳发光 (PulseCast)**：随节拍明灭的品红辉光，`发光强度` 建议 0.3–0.8。
4. 再拖一个 **怦然心率 (PulseCast)** 来源到画面角落显示 BPM 数字。

### 效果组合建议
| 场景 | 推荐 |
| --- | --- |
| 游戏恐怖/紧张时刻 | 抖动 8px + 发光 0.6，观众能"看见"你的紧张 |
| 聊天/日常 | 仅发光 0.3，轻微氛围不打扰 |
| 健身/运动直播 | 抖动 6px + 数字源大字，突出实时 BPM |

> 所有滤镜的 `服务地址/端口` 默认 `localhost:4567`，与叠层同源，改一处即可整体切换。

### 与浏览器源叠层的关系
- 想要**画面反应**（摄像头本体抖/发光）→ 用原生滤镜（本节）。
- 只想要**角落一个心率挂件**→ 浏览器源 `http://localhost:4567/obs?style=pill` 更轻量。
- 两者可同时用，数据完全一致。

---

## 二、VRChat 适配（OSC）

把实时心率通过 OSC 发送给 VRChat，驱动 avatar 参数与 ChatBox。

### 启用
```bash
# 独立服务方式
cargo run -p pulsecast-server -- --webroot . --vrchat --chatbox
# 自定义 VRChat OSC 地址（默认 127.0.0.1:9000）
cargo run -p pulsecast-server -- --vrchat --osc-addr 127.0.0.1:9000
```
桌面端后续会在「设置 → 叠层/联动」提供开关，无需命令行。

> VRChat 需在启动参数或 Radial Menu → Options → OSC 中**启用 OSC**。

### 发送的 avatar 参数（`/avatar/parameters/<名称>`）
| 参数 | 类型 | 范围 | 用途 |
| --- | --- | --- | --- |
| `HeartRate` | int | 0–255 | 原始 BPM |
| `HeartRateInt` | int | 0–255 | 别名，兼容不同 avatar |
| `HeartRateFloat` | float | -1.0–1.0 | `bpm/127-1`，0 中心映射（驱动混合形状/动画） |
| `HeartRatePercent` | float | 0.0–1.0 | `bpm/255`，直接驱动 shader/进度 |
| `isHRConnected` | bool | — | 是否已连接心率源 |
| `onesHR` / `tensHR` / `hundredsHR` | int | 0–9 | 个/十/百位，驱动三位数字翻牌 avatar |

### 在 avatar 里怎么用（举例）
- 用 `HeartRatePercent` 驱动一个 Emission 强度 → 心跳越快胸口发光越亮。
- 用 `onesHR/tensHR/hundredsHR` 各接一个 0–9 的数字贴图翻牌 → 头顶实时显示 BPM。
- 用 `isHRConnected` 控制整组心率显示的可见性（未连接时隐藏）。

### ChatBox
开启 `--chatbox` 后，每帧向 `/chatbox/input` 发送 `♥ 72 BPM`（立即显示、不响提示音），
在 VRChat 里表现为你头顶聊天框实时刷新心率。

> ⚠️ VRChat 对 ChatBox 有速率限制（约 1.3s/条）。当前 1Hz 推送基本安全；若被限流可在设置里降频。
