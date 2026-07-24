# PulseCast（怦然）技术诊断与架构演进评估报告（重验版）

> 全栈工程师 | 2026-07-22 | 基于工作树最新代码（base: b419201 + 未提交变更）
> 本版为追加重验，保留原诊断结论并标注已修复/新增变化。

---

## 〇、重验摘要：与原报告对比

| 原报告结论 | 当前状态 | 变化说明 |
|-----------|---------|---------|
| P0: WS/API 无认证 | **已修复（WS 层）** | 新增 token 认证（Tauri 启动生成 → IPC 下发 → WS ?token= 校验）+ Origin 白名单中间件 |
| P0: HTTP API 无认证 | **部分修复** | Origin guard 拦截跨域请求，但同机同 Origin 进程仍可调用 /api/source、/api/config |
| P1: 无 CI/CD | **已修复** | 新增 .github/workflows/ci.yml（test+clippy+build）+ release.yml（macOS dmg + Windows exe） |
| P2: glow filter uv_size 硬编码 | **已修复** | 现从 obs_source_get_base_width/height 动态获取，fallback 1920x1080 |
| P1: 设置不持久化 | **已修复** | 新增 server/src/config.rs + /api/config GET/POST 端点 |
| P0: Tauri mobile beta | 未变 | 仍为 beta，tauri.mobile.conf.json 配置不变 |
| P1: 前端无框架 | 未变 | 仍为 vanilla JS（settings 18KB、mobile.js 303行、obs-overlay 16KB） |
| P1: OBS 多实例重复 WS | 未变 | 每个 source/filter 独立 pc_ws_connect |
| P3: C 层 JSON 解析不健壮 | 未变 | strstr 模式 |
| 中: CSP 为 null | 未变 | tauri.conf.json + tauri.mobile.conf.json 均 csp:null |

---

## 一、代码质量评估

### 1.1 Rust Server（评分：8.5/10，原 8/10 ↑）

**文件清单：** lib.rs(866行) + ble.rs(277行) + osc.rs(196行) + config.rs(70行) + main.rs(74行) = 1,483行

**优点（保持）：**
- 类型安全、模块划分清晰（lib/ble/osc/config/main）
- 合理使用 tokio broadcast/watch 通道，双层通道设计（inner_tx 裸帧 → tx 标注帧）
- feature gate 隔离 BLE 依赖（无蓝牙环境零成本编译）
- 单元测试覆盖核心逻辑（evaluate、annotate、handle_command）
- SourceController supervisor 模式支持运行时热切换 Mock/BLE

**新增优点：**
- token 认证实现简洁（Opts.token → AppState → ws_handler Query 校验）
- Origin 白名单中间件（localhost + tauri:// 协议）
- config.rs 跨平台路径处理（macOS/Windows/Linux 三平台）
- CLI 参数设计完整（clap derive，支持 --token/--mock/--real/--vrchat/--hr-high/--hr-low）

**不足（保持）：**
- run_server() 仍承载初始化+路由+启动（~100行），可拆分为 build_router() + serve()
- broadcast channel 容量硬编码 16（高并发订阅者可能 Lagged）
- Sim 随机数用 LCG（可接受但非最佳）
- 缺少集成测试（WS 端到端、HTTP API 测试）

**新增不足：**
- config.rs 的 dirs_path() 未处理其他 Unix 平台（如 FreeBSD），仅 cfg 三分支
- token 生成用 DefaultHasher（非密码学安全），对本地场景可接受但不够理想
- /api/config POST 无请求体大小限制（axum 默认无 body limit）

### 1.2 Tauri Shell（评分：8/10，原 7/10 ↑）

**文件：** src-tauri/src/main.rs(61行) + tauri.conf.json + Cargo.toml

**优点（保持）：**
- 生产环境内嵌 Rust server 避免 Node 依赖
- release profile 优化到位（lto/strip/opt-level=s/codegen-units=1）

**新增优点：**
- WsToken 状态管理 + get_ws_token IPC command，前端可安全获取 token
- generate_token() 在 main() 中一次性生成，setup 中 move 进 server

**不足（保持）：**
- setup 中 unwrap_or_else 静默降级 resource_dir 错误为 "."
- CSP 仍为 null（tauri.conf.json:23）
- tray-icon feature 已声明但无逻辑实现

**新增不足：**
- generate_token() 用 DefaultHasher + SystemTime + pid，熵源有限（本地场景可接受）
- 开发模式下（debug_assertions）不启动 server 也不传 token，前端 dev 流程无认证

### 1.3 前端 JS（评分：6/10，未变）

**文件：** settings.html(18KB) + obs-overlay.html(16KB) + ftue.html(14KB) + mobile/mobile.js(303行)

**优点（保持）：**
- obs-overlay.html 自包含、支持 URL 参数配置、有 demo fallback
- mobile.js 结构清晰（IIFE + state 对象 + 三屏导航）

**不足（保持）：**
- 全部 vanilla JS 无框架，DOM 操作手动管理状态
- settings.html 与 mobile.js 逻辑重复（数据源切换、BLE 扫描、WS 连接）
- 无构建工具/类型检查/模块化
- ECG 动画 requestAnimationFrame 持续运行即使标签页不可见

**新增观察：**
- mobile.js wsUrl() 未附加 token 参数（`ws://${host}:${port}/ws`），若服务端启用 token 则移动端无法连接
- 前端无任何错误边界或全局异常处理

### 1.4 OBS C 插件（评分：7.5/10，未变）

**文件：** pc-ws.c(467行) + pulsecast-hr-source.c(342行) + pulsecast-glow-filter.c(274行) + pulsecast-shake-filter.c(176行) + plugin-main.c(28行) = 1,287行 + headers

**优点（保持）：**
- 零外部依赖（手写 WS 客户端+JSON 解析）
- 跨平台 socket/thread 封装完整（POSIX/Win32 双路径）
- pthread_mutex 保护共享状态，obs_queue_task 确保 UI 线程安全
- glow filter 新增 flash 远程控制联动（手机 → WS → 滤镜发光脉冲）

**已修复：**
- glow filter uv_size 不再硬编码，从 obs_source_get_base_width/height 动态获取

**不足（保持）：**
- JSON 解析为 strstr 模式，对嵌套/转义/Unicode 不健壮
- pc_read_handshake 逐字节 recv（最多 4096 次系统调用）
- WS 客户端不回 pong（注释说明依赖服务端不发 ping）
- 每个 source/filter 实例独立 WS 连接（3 个源 = 3 条 TCP + 3 个线程）
- pc_ws_destroy 先 close socket 再 join 线程存在竞态窗口
- Sec-WebSocket-Key 为固定值 "Vm1sdGlwQ2FzdA=="（不符合 RFC 6455 随机性要求）
- OBS 插件连接 WS 路径为 "/"，但服务端 WS 端点已迁移到 "/ws"（潜在兼容性问题）

---

## 二、架构合理性

### 2.1 数据流：BLE → Rust Server → WS Broadcast → 多端

架构清晰，保持不变：
```
心率源（BLE/Mock）→ inner_tx → transform（阈值注入）→ tx broadcast → WS 订阅者
                                                                    → OSC 转发
                                                                    → 控制事件（ctl_tx）
```

**新增：** /api/config 端点引入磁盘 I/O（同步 std::fs），在 axum 异步上下文中可能阻塞 tokio 工作线程（config.rs 用 std::fs 而非 tokio::fs）。

### 2.2 模块耦合度

- 低耦合保持：server/ble/osc/config 四模块通过 channel 或函数调用通信
- OBS 插件与 server 仅通过 WS 协议耦合（JSON 帧格式契约）
- **新增风险：** OBS 插件连接路径 "/" vs 服务端 "/ws" 不一致——需确认是否保留根路径 WS 升级（当前代码仅注册 /ws 路由）

### 2.3 错误处理

- Rust 层：anyhow + Context 链式传播良好；BLE 断线 3s 重试
- C 层：pc_worker 连接失败静默返回 NULL 无日志（保持）
- 前端：WS 断线有 demo fallback + 5s 重连（保持）
- **新增：** config::save() 返回 Result<(), String>（非 anyhow），错误信息为裸 io Error 字符串

---

## 三、跨平台兼容性风险

| 平台 | 风险等级 | 变化 | 说明 |
|------|---------|------|------|
| macOS | 低 | → | Tauri 2 + btleplug CoreBluetooth 成熟；CI 已覆盖 macOS 构建 |
| Windows | 中 | → | btleplug WinRT BLE（Win10 1803+）；OBS 插件 MSVC 编译；CI 已覆盖 Windows 构建 |
| Linux | 中-高 | → | btleplug 依赖 BlueZ D-Bus；CI 仅在 ubuntu 做 server 构建，未测 Tauri GUI |
| iOS | 高 | → | Tauri mobile beta；Web Bluetooth 在 iOS Safari 不可用 |
| Android | 高 | → | Tauri mobile beta；BLE 权限已声明但插件稳定性未验证 |

**新增观察：**
- release.yml 仅构建 macOS + Windows，Linux 桌面无发布包
- config.rs 的 Linux 路径 ~/.config/pulsecast 符合 XDG 规范（正面）
- CI 安装了 Tauri Linux 依赖（libwebkit2gtk-4.1-dev 等）但仅跑 test/clippy，未做 GUI 构建

---

## 四、性能瓶颈

### 4.1 BLE 轮询频率（无瓶颈，保持）
- btleplug 使用 OS 原生通知机制（subscribe characteristic），非轮询
- 心率设备通常 1Hz 推送

### 4.2 WS 广播延迟（保持）
- broadcast channel 容量 16：1Hz 帧率下充裕
- OBS 多实例各自独立 TCP 连接（N 源 = N 连接 + N 线程）——未改善
- **新增：** /api/config 同步文件 I/O 在异步 runtime 中可能短暂阻塞（影响极小，config 操作低频）

### 4.3 OBS 渲染（改善）
- glow filter uv_size 已动态化，非 1080p 画布不再异常
- 5x5 邻域采样（25 次 texture sample）在 4K 下仍有 GPU 压力（保持）
- HR 文字源 bpm_dirty 机制仅变化时更新（保持）
- ECG canvas requestAnimationFrame 60fps 持续运行（保持）

---

## 五、安全隐患（更新）

| 编号 | 问题 | 严重度 | 状态 | 说明 |
|------|------|--------|------|------|
| S1 | WS 无认证 | 高→低 | **已修复** | Tauri 模式：token 生成 → IPC 下发 → WS ?token= 校验；独立运行模式可 --token 指定 |
| S2 | HTTP API 无认证 | 高→中 | **部分修复** | Origin guard 拦截跨域；但同机 curl/脚本无 Origin 头可绕过（origin_guard 仅在 Origin 头存在时校验） |
| S3 | CSP 为 null | 中 | 未变 | tauri.conf.json + tauri.mobile.conf.json 均 csp:null |
| S4 | 端口固定 4567 | 低 | 未变 | 可预测但仅绑定 127.0.0.1 |
| S5 | innerHTML 拼接 | 低 | 未变 | obs-overlay.html 设备名拼入 HTML（数据来自本地 WS） |
| S6 | WS Key 硬编码 | 低 | 未变 | pc-ws.c Sec-WebSocket-Key 固定值 |
| S7 | Origin guard 可绕过 | 中 | **新增** | 无 Origin 头的请求（curl、同机 native 进程）不受 origin_guard 拦截 |
| S8 | /api/config 无鉴权 | 中 | **新增** | 同机进程可 GET/POST 用户配置（含设备名、阈值等），无 token 校验 |
| S9 | token 熵源不足 | 低 | **新增** | DefaultHasher(SystemTime+pid) 非 CSPRNG，本地场景可接受 |
| S10 | mobile.js 未适配 token | 中 | **新增** | wsUrl() 无 token 参数，服务端启用 token 后移动端无法连接 |

**缓解建议更新：**
- S2/S7/S8：对 /api/* 路由追加 token 校验（Query 或 Authorization header），或改为仅允许 Tauri IPC 调用
- S10：mobile.js 需从 Tauri IPC 或 URL 参数获取 token 后附加到 WS URL
- S3：设置合理 CSP（default-src 'self'; connect-src ws://localhost:4567）

---

## 六、技术债务清单（更新排序）

| 优先级 | 债务项 | 状态 | 影响 | 修复成本 |
|--------|--------|------|------|----------|
| P0 | Tauri mobile beta 依赖 | 未变 | 移动端无法稳定发布 | 高 |
| P0 | HTTP API + config 端点缺 token 校验 | 新发现 | 同机恶意读写配置/切换源 | 低 |
| P0 | mobile.js 未适配 token 认证 | 新发现 | 启用 token 后移动端断连 | 低 |
| P1 | 前端无框架/无模块化 | 未变 | 维护成本线性增长 | 中-高 |
| P1 | OBS 多实例重复 WS 连接 | 未变 | 资源浪费+扩展性差 | 中 |
| P1 | OBS 插件 WS 路径 "/" vs 服务端 "/ws" | 新发现 | 插件可能无法连接新版服务端 | 低 |
| P1 | CSP 为 null | 未变 | Tauri webview 无内容安全策略 | 低 |
| P2 | settings.html 与 mobile.js 逻辑重复 | 未变 | 改一处忘另一处 | 中 |
| P2 | Tauri tray-icon/热键未实现 | 未变 | 功能不完整 | 中 |
| P2 | config.rs 同步 I/O 在异步上下文 | 新发现 | 极低频，理论阻塞 | 低 |
| P3 | C 层 JSON 解析不健壮 | 未变 | 异常帧可能解析错误 | 中 |
| P3 | pc_ws 不回 pong | 未变 | 未来服务端启用 ping 则断连 | 低 |
| P3 | dev-server.mjs 与 Rust server 功能不对等 | 未变 | 开发/生产行为差异 | 低 |
| ~~P0~~ | ~~WS/API 无认证~~ | **已修复** | — | — |
| ~~P1~~ | ~~无 CI/CD~~ | **已修复** | — | — |
| ~~P2~~ | ~~glow filter uv_size 硬编码~~ | **已修复** | — | — |
| ~~P1~~ | ~~设置不持久化~~ | **已修复** | — | — |

---

## 七、技术演进建议（更新）

### 7.1 是否引入前端框架？
**建议保持：是，引入轻量框架（Svelte 或 SolidJS）**
- 理由不变：~1200 行 vanilla JS，状态管理靠手动 DOM
- 新增理由：mobile.js 需适配 token 认证、settings 需对接 /api/config——改动量增加使框架迁移 ROI 更高
- 迁移路径不变：settings → mobile → obs-overlay（OBS 保持独立）

### 7.2 是否需要插件化架构？
**建议保持：OBS 侧暂不需要，服务端可考虑**
- 新增观察：config.rs 的引入是良好起点，未来可按 trait 抽象输出插件（OSC/Twitch/YouTube）

### 7.3 移动端方案是否需替换 Tauri mobile？
**建议保持：短期 PWA，中期评估原生化**
- 新增紧迫性：mobile.js 未适配 token 认证，当前代码在 Tauri 桌面模式下移动端已无法连接
- PWA 方案需解决 token 传递（可通过 URL fragment 或 Tauri deep link）
- 建议 v1.0 先修复 mobile.js token 适配，保持 Tauri mobile 配置但标记为 experimental

### 7.4 安全加固路线图（更新）

| 阶段 | 措施 | 状态 |
|------|------|------|
| Phase 1 | WS token 认证 + Origin guard | ✅ 已完成 |
| Phase 2 | /api/* 追加 token 或改为 Tauri IPC only | 待做 |
| Phase 2 | mobile.js 适配 token | 待做 |
| Phase 3 | CSP 策略配置 | 待做 |
| Phase 3 | OBS 插件支持 token 参数（设置面板） | 待做 |
| Phase 4 | token 改用 getrandom/OsRng 生成 | 可选 |

### 7.5 其他演进建议（更新）

- **OBS 连接池（P1）**：引入进程内单例 WS 管理器，多 source/filter 共享一条连接——未变
- **OBS 插件 WS 路径修复（P1 新增）**：pc_ws_connect 的 path 参数应从 "/" 改为 "/ws"，或在服务端保留根路径 WS 升级兼容
- **统一开发服务（P3）**：废弃 dev-server.mjs，开发态直接 cargo run --mock——未变
- **测试覆盖（P1→P2 降级）**：CI 已建立基础（cargo test + clippy），下一步补充集成测试（axum test client + WS）
- **Linux 发布（P2 新增）**：release.yml 补充 Linux AppImage/deb 构建

---

## 八、总结

### 进步（本次重验确认）
1. 安全层面从"完全裸奔"提升到"WS 有 token + Origin 白名单"，P0 安全风险已大幅缓解
2. CI/CD 从零到可用（test + clippy + build + release），具备基础质量保障
3. 设置持久化已实现，用户体验断点修复
4. OBS glow filter 分辨率适配修复

### 仍需关注
1. **API 层认证缺口**（S2/S7/S8）：Origin guard 对无 Origin 头的同机请求无效，/api/config 完全开放
2. **移动端 token 适配断裂**（S10）：当前 mobile.js 在 Tauri 桌面模式下无法连接 WS
3. **OBS 插件 WS 路径不匹配**：插件连 "/"，服务端注册 "/ws"——需确认是否存在兼容路由
4. **Tauri mobile beta 风险**：移动端方案仍无明确 Plan B 落地
5. **前端无框架**：随着 token 适配、config 对接等新需求叠加，迁移 ROI 持续上升

### 整体技术健康度评分
- 原报告：6.5/10
- 本次重验：**7.5/10**（安全+CI+持久化三项修复带来显著提升）
- 达到"可公开发布"标准仍需：修复 API 认证缺口 + mobile token 适配 + OBS 路径兼容（约 1-2 天工作量）
