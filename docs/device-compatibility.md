# PulseCast 设备兼容性列表

> 最后更新：2026-07-26 · 与 `server/src/device_profiles.rs` 内置预设同步 · 社区验证 + 官方测试

## 状态说明
- ✅ 已验证可用
- ⚠️ 可用但有注意事项
- ❌ 不兼容
- 🔲 待验证

> 标注「需手机 App」的设备：必须先在手机配套 App 中开启心率广播，PulseCast 才能收到数据。
> 每款设备的完整分步引导（含排查建议）以软件内 FTUE 向导「步骤 1 · 连接设备」为准。

---

## 小米

### 小米手环 · 🔲 待验证
- **覆盖型号**：Mi Band / Xiaomi Smart Band / Redmi Band 系列（手环 8、9、9 Pro、NFC 版等）
- **手机 App 配置**：需要 — 小米运动健康（Mi Fitness）
- **开启心率广播**：小米运动健康 App → 设备 → 选择手环 → 心率 / 健康监测 → 开启「心率广播」（部分固件在 设备设置 → 心率 中）→ 确保手机 App 未独占 BLE 连接（可临时关闭 App 后台）
- **已知问题**：
  - 部分固件版本需重启手环后才能开启广播
  - 小米手环 7 及更早型号可能不支持 BLE 心率广播
  - 开启广播后手机 App 可能断开与手环的连接，属正常现象

### 小米/红米手表 · 🔲 待验证
- **覆盖型号**：Xiaomi Watch / Redmi Watch 系列（小米手表 S3、S4，Redmi Watch 等）
- **手机 App 配置**：需要 — 小米运动健康（Mi Fitness）
- **开启心率广播**：小米运动健康 App → 设备 → 选择手表 → 心率 / 健康监测 → 开启「心率广播」
- **已知问题**：
  - 小米手表 S3/S4、Redmi Watch 系列均需在 App 中开启广播
  - 开启广播后手机 App 可能断开连接，属正常现象

---

## 华为

### 华为手环/手表 · 🔲 待验证
- **覆盖型号**：HUAWEI Band（手环 8、9 等）、HUAWEI WATCH GT / FIT / Ultimate 系列（GT4、GT5、WATCH 4 等）
- **手机 App 配置**：需要 — 华为运动健康（Huawei Health）
- **开启心率广播**：华为运动健康 App → 设备 → 选择手环/手表 → 心率 → 开启「心率实时广播」（部分型号在 设备设置 → 心率 → 实时广播 中）；部分型号另需在手表端 设置 → 蓝牙 → 允许被其他设备发现
- **已知问题**：
  - GT 系列手表广播间隔可能为 2s，属正常现象
  - 需确保手环/手表未通过手机 App 独占 BLE 连接
  - WATCH 4 / Ultimate 系列需在 App 中确认开启实时广播

---

## 荣耀

### 荣耀手环/手表 · 🔲 待验证
- **覆盖型号**：Honor Band、Honor MagicWatch、Honor Watch 系列（如 Honor Band 9、MagicWatch 2、Watch GS 等）
- **手机 App 配置**：需要 — 荣耀运动健康（Honor Health，**注意不是华为运动健康**）
- **开启心率广播**：荣耀运动健康 App → 设备 → 选择手环/手表 → 心率 → 开启「心率广播」或「心率实时广播」
- **已知问题**：
  - 荣耀设备使用独立的「荣耀运动健康」App（非华为运动健康）
  - MagicWatch / Watch GS 系列广播间隔可能为 2s

---

## Amazfit（华米）

### Amazfit 华米手表 · 🔲 待验证
- **覆盖型号**：GTR / GTS 4 及以上、T-Rex 2 / T-Rex Ultra、Cheetah、Falcon、Balance、Active、Helio 系列
- **手机 App 配置**：不需要 — 多数型号即连即用（配套 App 为 Zepp，仅用于日常同步）
- **开启心率广播**：确保手表蓝牙开启即可；若扫描不到，在手表端 设置 → 心率 → 开启「心率广播 / Broadcast HR」（部分较新固件需要）
- **已知问题**：
  - 多数 Amazfit 手表即连即用，无需额外配置
  - 部分较新固件可能需要在手表设置中手动开启心率广播

---

## OPPO

### OPPO 手表/手环 · 🔲 待验证
- **覆盖型号**：OPPO Watch 系列、OPPO Band 系列
- **手机 App 配置**：需要 — OPPO 健康（HeyTap Health）
- **开启心率广播**：OPPO 健康 App → 设备 → 选择手表/手环 → 开启「心率广播」或「实时心率共享」
- **已知问题**：
  - OPPO Watch 系列需在 OPPO 健康 App 中开启心率广播
  - 具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」

---

## vivo

### vivo 手表/手环 · 🔲 待验证
- **覆盖型号**：vivo Watch 系列、vivo Band 系列
- **手机 App 配置**：需要 — vivo 健康
- **开启心率广播**：vivo 健康 App → 设备 → 选择手表/手环 → 开启「心率广播」或「实时心率」
- **已知问题**：
  - vivo Watch 系列需在 vivo 健康 App 中开启心率广播
  - 具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」

---

## 一加

### 一加手表/手环 · 🔲 待验证
- **覆盖型号**：OnePlus Watch 系列、OnePlus Band 系列
- **手机 App 配置**：需要 — OnePlus Health（一加健康）
- **开启心率广播**：OnePlus Health App → 设备 → 选择手表 → 开启「心率广播」或「实时心率共享」
- **已知问题**：
  - OnePlus Watch 系列需在 OnePlus Health App 中开启心率广播
  - 具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」

---

## Garmin（佳明运动表）

### Garmin 佳明运动表 · 🔲 待验证
- **覆盖型号**：Forerunner 245 及以上、Fenix 6 及以上、Venu 2 及以上、Epix、Instinct 2 及以上、Vivoactive、Enduro、Tactix 系列
- **手机 App 配置**：不需要 — 在手表端开启即可（配套 App 为 Garmin Connect，仅用于日常同步）
- **开启心率广播**：手表端 设置 → 传感器与配件（Sensors & Accessories）→ 心率 → 开启「广播心率」（Broadcast Heart Rate）；部分型号路径为 设置 → 活动与 App → 心率 → 广播心率
- **已知问题**：
  - 广播心率功能需手表端手动开启，默认关闭
  - 开启广播后手表电量消耗增加

---

## COROS（高驰）

### COROS 高驰运动表 · 🔲 待验证
- **覆盖型号**：PACE 2 / PACE 3、VERTIX 2、APEX 2 / APEX 2 Pro
- **手机 App 配置**：不需要 — 在手表端开启即可（配套 App 为 COROS）
- **开启心率广播**：手表端 设置 → 更多（More）/ 系统设置 → 开启「心率广播」（HR Broadcast）
- **已知问题**：
  - PACE 2/3、VERTIX 2、APEX 2/APEX 2 Pro 均支持心率广播
  - 广播功能需手表端手动开启

---

## Suunto（颂拓）

### Suunto 颂拓运动表 · 🔲 待验证
- **覆盖型号**：Suunto Race、Vertical、9 Peak Pro 等较新型号
- **手机 App 配置**：不需要 — 在手表端开启即可（配套 App 为 Suunto）
- **开启心率广播**：手表端 设置 → 连接（Connectivity）/ 一般设置（General）→ 开启「心率广播」（HR broadcast）
- **已知问题**：
  - Suunto Race、Vertical、9 Peak Pro 等较新型号支持心率广播
  - 广播功能需手表端手动开启，默认关闭

---

## Polar

### Polar 心率带 · 🔲 待验证
- **覆盖型号**：Polar H10、H9、H7
- **手机 App 配置**：不需要 — 佩戴后自动开机广播
- **开启心率广播**：佩戴胸带，确保电极片贴合皮肤（可用水微微润湿电极片）→ 等待约 5 秒，胸带自动开机并开始广播
- **已知问题**：
  - Polar H10/H9 同时支持 BLE 和 ANT+，PulseCast 使用 BLE
  - 胸带需贴合皮肤才能检测心率，悬空无数据

### Polar 运动手表 · 🔲 待验证
- **覆盖型号**：Vantage V2/M2、Grit X/Pro、Pacer/Pacer Pro、Ignite 3、Unite
- **手机 App 配置**：不需要 — 在手表端开启即可（配套 App 为 Polar Flow）
- **开启心率广播**：手表端 设置 → 一般设置（General settings）→ 开启「心率广播」（HR broadcast）
- **已知问题**：
  - Vantage V2/M2、Grit X/Pro、Pacer/Pacer Pro、Ignite 3 均支持
  - 广播功能需手表端手动开启

---

## Wahoo

### Wahoo TICKR 心率带 · 🔲 待验证
- **覆盖型号**：TICKR、TICKR X、TICKR Fit
- **手机 App 配置**：不需要 — 佩戴后自动开机广播
- **开启心率广播**：佩戴胸带，确保电极片贴合皮肤 → 胸带自动开机并开始 BLE 广播
- **已知问题**：
  - TICKR / TICKR X / TICKR Fit 均支持 BLE 心率广播
  - TICKR X 同时支持 BLE 和 ANT+

---

## Garmin HRM（佳明心率带）

### Garmin 心率带 · 🔲 待验证
- **覆盖型号**：HRM-Pro、HRM-Pro Plus、HRM-Dual、HRM-Fit
- **手机 App 配置**：不需要 — 佩戴后自动开机广播
- **开启心率广播**：佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）→ 胸带自动开机并开始 BLE 广播
- **已知问题**：
  - HRM-Pro / HRM-Pro Plus / HRM-Dual / HRM-Fit 均支持 BLE
  - HRM-Dual 同时支持 BLE 和 ANT+

---

## 迈金（Magene）

### 迈金 Magene 心率带 · 🔲 待验证
- **覆盖型号**：Magene H303、H64、L508
- **手机 App 配置**：不需要 — 佩戴后自动开机广播
- **开启心率广播**：佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）→ 胸带自动开机并开始 BLE 广播
- **已知问题**：
  - Magene H303 / H64 / L508 均支持 BLE 心率广播
  - 性价比高，国产胸带首选

---

## Coospo

### Coospo 心率带 · 🔲 待验证
- **覆盖型号**：Coospo H808S、H9Z 等型号
- **手机 App 配置**：不需要 — 佩戴后自动开机广播
- **开启心率广播**：佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）→ 胸带自动开机并开始 BLE 广播
- **已知问题**：
  - Coospo H808S / H9Z 等型号支持 BLE 心率广播

---

## Apple Watch

### Apple Watch · ⚠️ 实验性 / 非官方支持
- **覆盖型号**：Apple Watch 全系列（均需第三方 App 中转）
- **手机 App 配置**：需要 — **Apple Watch 原生不广播标准 BLE 心率，必须依赖第三方 App 中转**（如 HeartCast、Heart Rate Broadcast 等，部分可能付费或订阅）
- **开启心率广播**：在 Apple Watch 上安装第三方心率广播 App → 打开该 App 并按指引开启 BLE 心率广播 → 确保 iPhone 蓝牙开启且与手表已连接 → 在 PulseCast 中扫描（广播名可能显示为第三方 App 名称）
- **已知问题**：
  - 原生不广播标准 BLE 心率，必须依赖第三方 App 中转，非官方支持
  - v0.9 不保证 Apple Watch 兼容性，体验可能不稳定
  - 广播间隔通常为 2s 或更长，延迟较高

---

## 如何贡献验证结果

1. 使用 PulseCast 连接你的设备
2. 确认心率数据正常显示（OBS 叠层 + 移动端）
3. 记录：设备型号、固件版本、操作系统、广播间隔、遇到的问题
4. 在 GitHub Issues 中提交，格式：`[设备验证] 设备名 - 状态`

---

*以软件内 FTUE 引导为准，本文档为参考；欢迎社区在 GitHub Issues 提交验证结果。验证结果可能因固件版本、操作系统和蓝牙环境而异。*
