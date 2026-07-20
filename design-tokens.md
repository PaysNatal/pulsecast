# 怦然 PulseCast · Design Tokens

> 主播心率直播伴侣 —— 开源桌面应用设计交付物
> 画布文件：「心率直播伴侣-设计稿」（Ardot，fileId `705583149734972`）
> 主题：暗色为主（主播场景），含浅色表面态用于亮色任务栏/浅背景

## 1. 颜色 Color

### 品牌 / 强调
| Token | Hex | 用途 |
|---|---|---|
| `accent` | `#FF4D6D` | 主强调色（主播粉），CTA / 心率数字 / Logo |
| `accent-soft` | `rgba(255,77,109,0.16)` | 发光层 / 高亮背景 |

### 表面 Surface（暗主题）
| Token | Hex | 用途 |
|---|---|---|
| `bg-base` | `#0E1116` | 页面/画布底色 |
| `surface` | `#161B22` | 卡片 / 面板 |
| `surface-2` | `#1E2128` | 输入框 / 次级面板 |
| `surface-3` | `#2A2D35` | 悬浮 tooltip / 弹层 |
| `border` | `#3A3F4A` | 分隔线 / 未激活控件 |

### 文本 Text
| Token | Hex | 用途 |
|---|---|---|
| `text-primary` | `#E8EAED` | 主文字 |
| `text-secondary` | `#9BA1A8` | 次要文字 / 标签 |
| `text-tertiary` | `#6B7280` | 占位 / 说明 |

### 浅面（亮态场景）
| Token | Hex | 用途 |
|---|---|---|
| `surface-light` | `#F5F6F8` | 浅色任务栏托盘背景 |
| `text-on-light` | `#18191C` | 浅背景上的文字 |

### 连接状态色 Status（替代原心率区间）
| Token | Hex | 含义 |
|---|---|---|
| `status-connecting` | `#9BA1A8` | 连接中（灰） |
| `status-live` | `#36C28C` | 实时（绿） |
| `status-device-lost` | `#F2994A` | 设备丢失（橙） |
| `status-error` | `#EB5757` | 错误（红） |

### 国内直播平台色
| Token | Hex | 平台 |
|---|---|---|
| `douyin` | `#FF4D6D` | 抖音 |
| `kuaishou` | `#FF6B35` | 快手 |
| `bilibili` | `#00AEEC` | B站 |
| `douyu` | `#FF9500` | 斗鱼 |
| `huya` | `#FF3B3B` | 虎牙 |

## 2. 字体 Typography
- **中文**：`Sarasa Gothic SC`（Bold / Semi Bold / Regular）
- **数字 / 英文**：`Inter`（Bold / Regular）
- 大号 BPM 数字：`Inter Bold` 42–72px
- 区块标题：`Sarasa Gothic SC Semi Bold` 16–18px
- 正文 / 标签：`Sarasa Gothic SC Regular` 12–14px

## 3. 圆角 Radius
- 卡片 / 面板：`12–16`
- 心率叠层药丸：`32–44`
- 圆点 / 图标按钮：`8–13`
- 开关（toggle）：高 `24`、圆角 `12`、旋钮 `18`

## 4. 间距 Spacing
- `xs=8` · `sm=10–12` · `md=14–16` · `lg=24`

## 5. 数据字段约定（前端 ← 本地 BLE 服务 / 设备）
```ts
interface HeartRateState {
  bpm: number;                                  // 实时心率；无数据时 0
  status: 'connecting' | 'live' | 'device_lost' | 'error';
  device: {
    name: string;                               // 例："Amazfit GTR 4"
    type: 'watch' | 'strap' | 'phone';
    signal: 'strong' | 'medium' | 'weak';
  };
  message?: string;                             // 错误/提示，如"请在手表开启心率广播"
}
```
> 已移除 `zone` / 心率区间字段——产品只为直播显示实时心率，不做区间分级。

## 6. OBS 浏览器源
- 地址：`http://localhost:4567/obs`
- 透明背景（Transparent background）
- 3 种样式：`心跳药丸` / `ECG 波形` / `极简大数字`

## 7. 设计板清单（画布）
1. 设计系统 Design System（`2:1`）—— 品牌 / 强调色 / 连接状态色 / 字体表面
2. 核心悬浮叠层 Core Overlay（`2:46`）—— 心跳 + ECG 两样式 + 4 状态胶囊
3. OBS 原生插件 OBS Plugin（`2:87`）—— 一键安装 CTA + 4 能力 + OBS 预览
4. 设备一键连接 FTUE 向导（`4:1`）—— 三步：选设备 → 扫描连接 → 预览开播
5. 设备兼容矩阵 Device Matrix（`4:79`）—— 本地直连 / HypeRate云 / 胸带 / 直播平台
6. OBS 浏览器源卡片（`7:2`）—— 地址栏 + 3 样式变体（**样式三已修复透明背景**）
7. 设置面板 Settings（`7:35`）—— 侧边导航 + 设备连接区
8. 托盘与菜单栏 Tray（`7:70`）—— Win 托盘 / macOS 菜单栏 / 右键菜单
9. 落地页 Landing（`7:118`）—— Hero + 特性 + 设备 + 下载
10. 图标集 Icons（`7:187`）—— App 图标 + 托盘 + 状态 + Logo 变体
11. 未来模块草图 Roadmap（`7:251`）—— VRChat/OSC + 摄像头联动（路线图，非 MVP）
