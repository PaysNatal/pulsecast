# obs-pulsecast · 怦然 OBS 原生模块

把「怦然 PulseCast」的实时心率直接搬进 OBS Studio 的画面里——**不需要浏览器源**，由 libobs 原生渲染，性能更好、与场景深度联动。

> 数据源 = 桌面主程序（或 `server/` 内嵌 Rust 服务）在 `ws://localhost:4567` 广播的心率帧，契约与 `obs-overlay.html` 完全一致：`{"bpm":72,"status":"live","device":{...},"message":""}`。

## 提供的能力

| 类型 | 名称 | 说明 |
| --- | --- | --- |
| 来源 Source | 怦然心率 (PulseCast) | 实时显示当前 BPM，心跳时字号轻微“怦然”缩放 |
| 滤镜 Filter | 心跳镜头抖动 | 每次心跳快速衰减的位移抖动，强化临场感 |
| 滤镜 Filter | 心跳发光 | 随节拍明灭的品红辉光（#FF4D6D）；收到 `flash` 控制事件时叠加一段高光白色脉冲 |
| 阈值动作（心率源属性） | 场景切换 | bpm 越过高/低阈值时，在 OBS 进程内自动切换指定场景（零外部依赖） |
| 远程控制（心率源 WS 监听） | 手机指挥 | 手机「控制中心」发来的 `switch_scene` / `flash` / `set_threshold` 控制事件，在 OBS 进程内真实执行 | |

三者都订阅同一个本地 WS，无需重复配置设备。阈值动作已实现：在「怦然心率」源的属性里配置高/低阈值（BPM）与目标场景名，
bpm 越界时由该源在 OBS 进程内直接调用 `obs_frontend_set_current_preview_scene2` 切换场景（不影响录制）。

## 远程控制（手机控制中心）

「怦然」移动端 App 的「控制中心」可通过 WebSocket 向桌面端服务发送 `type:"control"` 事件，由 **本原生心率源在 OBS 进程内真正执行**——这是浏览器叠层（`obs-overlay.html`）做不到的，浏览器源只能显示横幅提示，没有切场景的权限。

支持的控制命令（`kind`）：

| kind | 字段 | 原生模块行为 |
| --- | --- | --- |
| `switch_scene` | `scene`（场景名，支持中文） | 在 OBS 主线程调用 `obs_frontend_set_current_preview_scene2` 切换预览场景（不影响录制） |
| `flash` | — | 同时驱动两类原生模块：① 心率文字源 800ms 高光闪烁（放大 + 脉冲）；②「心跳发光」滤镜叠加一段向白色靠拢的高光辉光脉冲（峰值由 `flash_strength` 控制，默认 1.6） |
| `set_threshold` | `high` / `low` | 实时更新本源的高/低阈值，越阈场景切换随之生效（无需重启 OBS） |

> 控制事件与心率帧共用同一条 WS 链路：服务端把移动端命令广播给所有订阅者，原生源识别出 `type:"control"` 后分流处理，普通心率帧继续走 BPM 渲染。
> 场景切换通过 `obs_queue_task(OBS_TASK_UI, …)` 派发到主线程，避免在 WS 后台线程直接调用 frontend API 导致的未定义行为。

> **原生滤镜联动（无跨源 IPC）**：心率源、「心跳发光」滤镜各自独立订阅同一 WS 流，都消费 `type:"control"` 的 `flash` 事件——手机点一下「高光闪烁」，心率文字与摄像头/头像辉光在同一帧内同时亮起，彼此不需要任何进程内通信。解析统一使用 `pc_ws_parse_control_kind()`（`pc-ws.c`），契约一处定义、处处复用。
> 服务端广播格式（已修复）：`{"type":"control","kind":"flash","source":"mobile","at":<ms>}`，**必须带 `type` 字段**，原生 C 代码据此与心率帧区分。

## 构建（需 OBS Studio 开发头文件）

**macOS**
```bash
brew install obs
cd obs-pulsecast
cmake -B build -Dlibobs_DIR=$(brew --prefix obs)/lib/cmake/libobs
cmake --build build
# 安装到用户插件目录
cp build/obs-pulsecast.so ~/Library/Application\ Support/obs-studio/plugins/
```

**Windows（vcpkg）**
```bat
vcpkg install obs
cmake -B build -Dlibobs_DIR=<vcpkg>/installed/x64-windows/share/libobs
cmake --build build --config Release
```

**Linux**
```bash
# 安装 obs-studio-dev 或指向 obs-studio 源码树 (-DOBS_SOURCE_DIR=...)
cmake -B build && cmake --build build
```

## 使用
1. 启动怦然桌面端（或 `cargo run -p pulsecast-server -- --webroot .`）。
2. OBS 来源里添加「怦然心率」即可看到 BPM；给摄像头/头像源挂上「心跳镜头抖动」「心跳发光」滤镜。
3. （可选）在「怦然心率」源属性里设置心率高/低阈值与「越阈切换到的场景名」：心率越界时 OBS 自动切到该场景。
   阈值需在服务端开启（`cargo run -p pulsecast-server -- --webroot . --hr-high 150 --hr-low 55`），否则源收不到 `trigger` 事件。

> 阈值场景切换依赖 OBS frontend API（`obs_frontend_set_current_preview_scene2`，OBS 28+）。
> 构建时 `CMakeLists.txt` 已链接 `OBS::obs-frontend-api`；旧版 OBS 可把该调用改为 `obs_frontend_set_current_scene` 并相应调整链接。

## 与浏览器源的关系
- `obs-overlay.html`（浏览器源）**零安装、即拖即用**，适合快速上手与高级 CSS 定制。
- `obs-pulsecast`（原生模块）**性能更优、可与场景滤镜链深度组合**，适合做精致的直播间效果。

两条路共用同一份心率数据，按需选用。
