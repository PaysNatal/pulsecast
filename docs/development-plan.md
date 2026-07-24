# PulseCast（怦然）发展路线图

> 修订日期：2026-07-24 · v0.8 实施后更新
> 核心命题：**让恐怖游戏主播的心率飙升成为直播间最炸裂的 3 秒钟**

---

## v0.8 已交付（本次）

### 服务端
| 模块 | 文件 | 内容 |
|------|------|------|
| OBS WebSocket 客户端 | `server/src/obs_ws.rs` | OBS WS v5 认证 + CreateInput/SetInputSettings/RemoveInput/TriggerReplayBuffer |
| 设备 Profile | `server/src/device_profiles.rs` | 小米/华为/Amazfit/Polar/Wahoo 五种设备引导 |
| 梯度引擎 | `server/src/lib.rs` | HrFrame.intensity (0~1) + ExternalEvent 预留（Osc/LiveRoom） |
| 名场面系统 | `server/src/highlights.rs` | 30 分钟 ring buffer + spike 检测 + /api/highlights |
| API 新增 | `server/src/lib.rs` | /api/obs/status, /api/obs/inject, /api/obs/remove, /api/highlights, /api/ble/profiles |

### 前端
| 页面 | 内容 |
|------|------|
| FTUE | 四步向导：设备 Profile → 预览+样式切换 → OBS 一键注入 → 开播检查 |
| 设置页 | OBS WebSocket 地址/密码 + 一键注入/删除按钮 |
| OBS 叠层 | intensity 驱动数字颜色梯度（白→粉→红）+ 微放大 |

### Tauri 桌面端
- 系统托盘（左键显示窗口，右键菜单）
- 配置零感知：启动自动重连上次设备 + 自动更新 OBS 源 URL

### OBS 原生插件
- 新增「心率氛围」全屏滤镜（GLSL：vignette + 色温偏移 + 边缘脉冲 + 全屏闪烁）
- 已注册到 plugin-main.c + CMakeLists.txt

### 架构预留
- `ExternalEvent::LiveRoom` 枚举（弹幕/礼物联动接口，v1.x 实现）
- `config.live_room_url` 字段（直播间地址）
- WS 广播帧含 `external_events` 字段（当前为空数组）

---

## v0.9 · 打磨与扩展

### 核心：让效果更炸裂
- **氛围滤镜参数化**：设置页控制 vignette 强度、色温范围、脉冲频率、闪烁颜色
- **更多特效主题**：「电竞红」「赛博霓虹」「像素复古」三套预设
- **OBS 连接池**：进程内单例 WS（pc-ws-manager.c），N 个源/滤镜共享 1 条连接
- **glow/shake 梯度适配**：接收 intensity 而非仅 bpm，低心率时效果休眠

### 设备兼容性
- 真机验证矩阵：小米手环 8/9、华为 GT4、Amazfit GTR4、Polar H10
- 断连预测：RSSI 持续下降 → 提前警告「信号弱」
- FTUE 设备引导截图（每种设备的具体 App 设置步骤）

### 性能
- ECG 离屏渲染（OffscreenCanvas + Worker）
- 氛围滤镜降采样（1/4 分辨率做 glow 再叠加）
- 服务端 `tokio::fs` 替代 `std::fs`

---

## v1.0 · 公开发布

### 核心：让心率创造可传播的内容
- **名场面导出**：JSON + SVG 心率曲线卡片，主播发 B 站动态/切片时附带
- **OBS Replay Buffer 联动**：spike 触发时自动保存前后 30s（需用户预开启）
- **移动端名场面 tab**：本场所有 spike 事件列表 + 分享文案一键复制

### 分发
- GitHub Release v1.0（macOS dmg + Windows exe + Linux AppImage）
- B 站教程视频：「5 分钟在直播里显示心率」
- 即刻 Build in Public 系列
- 种子用户招募：5 个小米手环 + 5 个华为手环寄给 B 站小主播

### 质量
- 设备兼容性列表页面（GitHub Wiki）
- CI 全绿（已有）+ Release 自动化（已有）
- CSP 策略配置（Tauri 安全加固）

---

## v1.x · 生态

### 弹幕/礼物联动（架构已预留）
- 新建 `server/src/live_room.rs`：连 B 站/抖音直播间 WebSocket
- 解析弹幕/礼物 → `ExternalEvent::LiveRoom` → 广播帧
- 前端/插件消费 LiveRoom 事件触发特效（已预留消费接口）
- 观众送礼物 → 触发「惊吓特效」→ 主播心率真的飙了 → 正反馈循环

### 社区
- 叠加样式模板市场（GitHub Discussions）
- 「本周最佳心率叠层」社区投票
- 国际化：日语（VTuber）+ 韩语（游戏直播）

### 架构演进
- 前端 Svelte 迁移（settings → mobile → ftue）
- 移动端 Tauri Mobile（配置已存在）
- 插件化输出（OutputPlugin trait：OSC / OBS WS / Twitch / YouTube）

---

## 不可违背的约束

1. **本地优先。** 不引入云端依赖，不上传心率数据，不做账号系统。
2. **免费开源。** MIT 许可证，所有代码公开。
3. **OBS 兼容。** 核心体验围绕 OBS 构建。
4. **蓝牙直连。** 不依赖手机 App 中转。
5. **一人节奏。** 修完 → 测 → 发 → 收反馈。
6. **性能预算。** 任何新功能不得使 CPU 占用增加超过 0.5%。

---

*每个版本独立可交付，不互相阻塞。优先级以"主播能不能用起来"为唯一判据。*
