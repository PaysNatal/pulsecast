# PulseCast（怦然）— 项目交接文档

| 字段 | 值 |
|------|-----|
| 编制日期 | 2026-07-23 |
| 编制人 | QoderWork（本次会话） |
| 项目路径 | `/Users/locodocoww/WorkBuddy/PulseCast` |
| Git 仓库 | `github.com/PaysNatal/pulsecast`（私有） |
| 当前 commit | `b419201`（Initial commit，唯一提交） |
| 工作区状态 | 有未提交的修改（安全/CI/配置相关，在 b419201 之上） |

---

## 一、30 秒了解这个项目

PulseCast 是一个**免费开源的主播心率直播工具**。主播戴蓝牙手环/手表，软件通过 BLE 读取心率，在 OBS 直播画面上显示实时心率叠加层，并支持心跳触发 OBS 特效（发光、抖动、场景切换）。

核心卖点：完全本地（零云端）、OBS 原生 C 插件提供 GPU 加速特效、中文优先、适配小米/华为手环。

**项目不是从零开始。** 代码已经写了大约 85%，核心功能全部实现。卡在 3 个 Bug 和真机验证上。

---

## 二、技术架构

```
BLE 手环/手表 (0x180D)
       │
       ▼
┌─────────────────────────────────┐
│  Rust Server (axum+tokio+btleplug) │  ← 端口 :4567
│  模块: ble.rs / osc.rs / config.rs │
│  功能: BLE连接 / 阈值引擎 / WS广播  │
└───┬─────┬─────┬─────┬─────┬─────┘
    │     │     │     │     │
    ▼     ▼     ▼     ▼     ▼
  Tauri  OBS   OBS   手机   VRChat
  桌面端 浏览器 原生  遥控器  OSC
  (设置) 叠加  C插件  (PWA)  UDP:9000
```

| 层 | 技术 | 关键文件 |
|----|------|---------|
| 后端 | Rust: axum 0.8, tokio, btleplug 0.12 (feature-gated) | `server/src/{lib,main,ble,osc,config}.rs` (~1,483行) |
| 桌面壳 | Tauri v2 (2.6.3) | `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json` |
| 前端 | 原生 HTML/CSS/JS（无框架） | `ftue.html`, `settings.html`, `obs-overlay.html` |
| OBS 插件 | C (libobs, 零外部依赖, 手写 WS 客户端) | `obs-pulsecast/src/*.c/*.h` (~1,287行) |
| 手机端 | PWA (原生 JS) | `mobile/{index.html,mobile.js,mobile.css}` |
| CI/CD | GitHub Actions | `.github/workflows/{ci,release}.yml` |
| 设计 | Design tokens 体系 | `design-tokens.css`, `design-tokens.md` |

Cargo workspace: 根 `Cargo.toml` 包含 `server` + `src-tauri` 两个 member。

---

## 三、已实现的功能（全部有代码）

| 功能 | 状态 | 说明 |
|------|------|------|
| BLE 心率连接 | ✅ 可用 | 标准 HR Profile (0x180D/0x2A37)，uint8/uint16 解析，5个单元测试 |
| 模拟心率源 | ✅ 可用 | 无设备时预览效果 |
| OBS 浏览器叠加 | ⚠️ 需修 | 3种样式（心率胶囊/ECG波形/极简数字），透明背景。**缺 token** |
| OBS 原生 C 插件 | ⚠️ 需修 | 心率文字源 + 心跳抖动滤镜 + 心跳发光滤镜(GLSL)。**WS 路径错误** |
| 阈值引擎 | ✅ 可用 | 心率超阈值 → OBS 场景切换 + 警报脉冲，有单元测试 |
| VRChat OSC | ✅ 可用 | 8个 Avatar 参数 + ChatBox，3个单元测试 |
| 手机遥控器 | ⚠️ 需修 | 局域网远程调阈值/切场景/闪光。**缺 token** |
| 设置持久化 | ✅ 可用 | JSON 配置文件，`/api/config` 读写 |
| 首次使用引导 | ✅ 可用 | 3步向导（连设备→配OBS→开播），可优化 |
| WS token 认证 | ⚠️ 部分 | WS 有 token，但 API 端点和前端页面未适配 |
| CI/CD | ✅ 可用 | macOS dmg + Windows exe 构建 |

---

## 四、3 个阻塞性 Bug（v0.9 发布前必须修）

### Bug 1: OBS 插件 WS 路径错误

- **文件**: `obs-pulsecast/src/pc-ws.c`
- **问题**: 所有三个 OBS 源/滤镜调用 `pc_ws_connect(host, port, "/", ...)` 连接 WS 路径 `/`
- **原因**: axum 0.7→0.8 迁移时，服务端 WS 路由从 `/` 改到了 `/ws`，但 C 插件没跟着改
- **修复**: 将 `pc-ws.c` 中的路径 `"/"` 改为 `"/ws"`（搜索 `pc_ws_connect` 调用处）
- **验证**: 编译插件后在 OBS 中添加心率源，确认能收到 BPM 数据

### Bug 2: 前端页面缺 token 传递

- **文件**: `mobile/mobile.js` 的 `wsUrl()` 函数；`obs-overlay.html` 的 WS 连接逻辑
- **问题**: WS URL 是 `ws://${host}:${port}/ws`，没有 `?token=xxx` 参数
- **原因**: Tauri release 模式下服务端强制 token 认证，但前端页面不知道 token
- **修复方案**:
  - Tauri 内嵌页面：通过 Tauri IPC (`get_ws_token` command) 获取 token，拼入 WS URL
  - OBS 浏览器源：在 URL 参数中传递 token（如 `http://localhost:4567/obs?style=pill&token=xxx`），obs-overlay.html 从 URL 读取
  - 手机遥控器：设置页面增加 token 输入框，或从服务器 API 获取
- **验证**: Tauri release 构建后，OBS 叠加页和手机遥控器都能连上 WS

### Bug 3: API 端点无 token 校验

- **文件**: `server/src/lib.rs` 的路由定义
- **问题**: `/api/source`, `/api/config`, `/api/ble/scan` 只有 `origin_guard` 中间件，该中间件仅在请求带 `Origin` 头时校验。同机 `curl` 或原生进程（无 Origin 头）可绕过
- **修复**: 为 `/api/*` 路由添加 token 校验中间件（从 query param 或 header 读取 token，与服务端生成的 token 比对）
- **验证**: `curl http://localhost:4567/api/config` 应返回 401；带正确 token 应返回 200

**总预估工时: 6-7 小时**

---

## 五、真机验证清单（发布硬门槛）

代码从未在真实硬件上运行过。以下验证必须在发布前完成：

| 验证项 | 所需设备 | 验证内容 |
|--------|---------|---------|
| BLE 连接 | 小米手环 + 华为手环（各1） | 开启心率广播后能被扫描、连接、持续收到 BPM |
| OBS 浏览器叠加 | 安装 OBS 的电脑 | 添加浏览器源 `http://localhost:4567/obs?style=pill`，看到心率显示 |
| OBS 原生插件 | OBS + OBS SDK 头文件 | 编译插件（CMake），添加心率源/抖动滤镜/发光滤镜，确认效果 |
| Tauri 桌面端 | macOS 或 Windows | `cargo tauri build`，打开 App，完成 FTUE 流程 |
| 手机遥控器 | 手机（同局域网） | 浏览器打开手机遥控页面，能调阈值/切场景 |
| 阈值触发 | 以上全部 | 心率超过阈值时 OBS 场景自动切换 |

**注意**: OBS 原生插件编译需要 OBS 开发头文件（`libobs`, `obs-frontend-api`）。macOS 可通过 Homebrew 安装 OBS 或从 OBS 官网下载 SDK。Windows 需要 Visual Studio + OBS SDK。

---

## 六、战略决策（已与产品负责人确认）

| 编号 | 决策 | 理由 |
|------|------|------|
| SD-001 | 不设固定发布日期，修完 Bug + 真机验证通过就发 | 质量优先；此前所有时间线估算均未兑现 |
| SD-002 | 真机验证是发布硬门槛 | 沙箱编译通过 ≠ 用户可用 |
| SD-003 | 度量 = GitHub 指标（Star/下载/Fork/Issue），不做应用内遥测 | 与本地优先定位一致 |
| SD-004 | 远期"直播辅助套件"暂不规划，不为它做架构预留 | 避免过度工程化 |
| SD-005 | 一人节奏推进，不套团队协作模板 | QoderWake 7角色团队的失败证明套模板无效 |

---

## 七、产品方向（已确认，详见 `docs/产品方向定义.md`）

- **定位**: 免费、本地优先、中文优先的主播心率直播伴侣
- **目标用户**: 游戏主播(P0) > VTuber/VRChat(P1) > 健身主播(P2)
- **差异化**: OBS 原生 GPU 特效 / 零云端隐私 / 阈值互动引擎 / 国产手环适配
- **商业模式**: 完全免费开源（MIT 或同等许可）
- **竞品**: HypeRate(云端+Twitch中心) / obs.cn插件(仅数字) / PulseOverlay(Python无release)。**国内市场空白**
- **明确不做**: 云端/账号/付费/ANT+/Apple Watch(v0.9)/心率数据记录回放

---

## 八、版本路线图

```
v0.9（当前）              v1.0                    v1.x
修3个Bug                叠加样式自定义            心率互动游戏
真机验证                设备扫描优化              更多OBS特效
FTUE优化               多语言完善                系统托盘+热键
中文README              手机遥控器完善            社区模板
GitHub Release          B站教程视频              套件集成（远期）
```

---

## 九、项目文件地图

```
PulseCast/
├── Cargo.toml              # Workspace 根配置
├── server/                 # Rust 后端
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # 入口，CLI 参数解析
│       ├── lib.rs          # 核心：路由、WS广播、阈值引擎、API
│       ├── ble.rs          # BLE 连接（btleplug，feature-gated）
│       ├── osc.rs          # VRChat OSC 输出
│       └── config.rs       # JSON 配置持久化
├── src-tauri/              # Tauri v2 桌面壳
│   ├── src/main.rs         # token 生成 + 服务器启动
│   ├── tauri.conf.json     # 窗口/构建/CSP 配置
│   └── Cargo.toml
├── obs-pulsecast/          # OBS 原生 C 插件
│   ├── CMakeLists.txt
│   ├── src/
│   │   ├── plugin-main.c   # 插件入口，注册3个源/滤镜
│   │   ├── pulsecast-hr-source.c/h   # 心率文字源 + 场景切换
│   │   ├── pulsecast-shake-filter.c/h # 心跳抖动滤镜
│   │   ├── pulsecast-glow-filter.c/h  # 心跳发光滤镜(GLSL)
│   │   └── pc-ws.c/h       # 手写 WS 客户端 ← Bug 1 在这里
│   └── test/test_pcws.c    # C 单元测试
├── obs-overlay.html        # OBS 浏览器叠加页 ← Bug 2 涉及
├── settings.html           # 设置页面
├── ftue.html               # 首次使用引导
├── mobile/                 # 手机遥控器 PWA ← Bug 2 涉及
│   ├── index.html
│   ├── mobile.js
│   └── mobile.css
├── design-tokens.css       # 设计 token 体系
├── dev-server.mjs          # Node 开发服务器（参考实现）
├── build-frontend.mjs      # 前端构建脚本
├── dist/                   # 构建输出
├── docs/                   # 项目文档
│   ├── 产品方向定义.md      # ★ 产品战略文档（本次会话产出）
│   ├── tech-diagnosis-report-v2.md  # 技术诊断（之前AI产出）
│   ├── comprehensive-review-report.md # 综合评审（之前AI产出）
│   ├── growth-strategy.md  # 增长策略（之前AI产出）
│   └── camera-and-vrchat.md # 摄像头+VRChat使用指南
├── .github/workflows/      # CI/CD
│   ├── ci.yml              # 测试 + clippy + 构建
│   └── release.yml         # macOS dmg + Windows exe
└── .workbuddy/memory/      # 之前开发会话的工作日志
```

---

## 十、接手后该做什么（优先级排序）

### 第一步：修 3 个 Bug（见第四节，~6-7h）

按 Bug 1 → Bug 3 → Bug 2 的顺序修。Bug 1 最简单（改一个字符串），Bug 3 次之（加中间件），Bug 2 最复杂（涉及多个前端页面 + Tauri IPC）。

### 第二步：本地构建验证

```bash
# 后端
cd /Users/locodocoww/WorkBuddy/PulseCast
cargo test --features ble    # 12个单元测试应全部通过
cargo build --features ble   # 编译服务器

# Tauri 桌面端
cargo tauri build            # 需要 Tauri CLI 和系统依赖

# OBS 插件（需要 OBS SDK）
cd obs-pulsecast
mkdir build && cd build
cmake .. -Dobs_DIR=<OBS SDK路径>
make
```

### 第三步：真机验证（见第五节）

### 第四步：FTUE 优化 + 中文 README

- FTUE 中补充 OBS 配置的图文引导（截图 + 步骤）
- README 确保中文说明完整，包含 demo GIF

### 第五步：GitHub Release v0.9

- 打 tag，触发 release.yml 构建
- 写 Release Notes（中文为主）
- 仓库从 private 改为 public

### 第六步：发布后

- B站发教程视频
- 即刻/V2EX 发帖
- 收集反馈，修 Bug，迭代

---

## 十一、已知技术债（v0.9 后处理，不影响发布）

| 问题 | 文件 | 优先级 |
|------|------|--------|
| 前端无构建工具/类型检查，逻辑在 settings.html 和 mobile.js 间重复 | 全局 | P2 |
| OBS 插件 JSON 解析用 strstr，不支持嵌套/转义/Unicode | `pc-ws.c` | P2 |
| token 用 DefaultHasher 生成，非密码学安全 | `src-tauri/src/main.rs` | P3 |
| Tauri CSP 为 null | `tauri.conf.json` | P2 |
| 无集成测试 | 全局 | P2 |
| 每个 OBS 源/滤镜各开一个 WS 连接（3个源=3条TCP） | `obs-pulsecast/` | P2 |
| `config.rs` 在 async handler 中用同步 `std::fs` | `server/src/config.rs` | P3 |
| broadcast channel 容量硬编码 16 | `server/src/lib.rs` | P3 |
| Tray icon / 全局热键声明了但未实现 | `src-tauri/Cargo.toml` | P3 |

---

## 十二、关键约束（不可违背）

1. **本地优先。** 不引入云端依赖，不上传心率数据，不做账号系统。
2. **免费开源。** MIT 或同等许可证。
3. **OBS 兼容。** 核心体验围绕 OBS 构建。
4. **蓝牙直连。** 不依赖手机 App 中转。
5. **一人节奏。** 不套团队流程，修完→测→发→收反馈。

---

## 十三、之前 AI 会话的产出评估

| 来源 | 产出 | 评价 |
|------|------|------|
| WorkBuddy (7月19-20日) | 全部代码 + 4份docs | **核心贡献。** 从零搭建了完整项目，代码质量 7.5-8.5/10 |
| QoderWake 团队 (7月22日) | 5份 Markdown（已删除→废纸篓） | **无效产出。** 未找到代码，基于"项目为零"的错误前提写了1200行模板 |
| QoderWork (7月23日，本次) | 产品方向定义.md + 本 HANDOFF | 产品战略梳理 + 竞品分析 + 交接文档 |

---

## 十四、环境信息

- **开发机**: macOS (darwin 25.5.0, arm64)
- **Rust**: 已安装（项目用 rustc 1.97.1）
- **Node**: 已安装（dev-server 用）
- **OBS**: 需确认是否已安装
- **蓝牙手环**: 用户有部分设备（具体型号待确认）
- **GitHub**: `github.com/PaysNatal/pulsecast`（私有仓库）

---

*本文档编制于 2026-07-23。接手者读完此文档后应能直接开始修 Bug，无需重新探索项目。如有疑问，优先查阅 `docs/` 下的对应文档。*
