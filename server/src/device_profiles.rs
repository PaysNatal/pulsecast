//! 设备 Profile —— 为已知 BLE 心率设备提供适配引导
//!
//! 每种设备有不同的广播行为、配置要求和已知问题。
//! FTUE 根据用户选择的设备类型展示对应的设置步骤，
//! BLE 扫描结果按 Profile 分组显示。

use serde::Serialize;

/// 设备 Profile：描述一种已知 BLE 心率设备的特征和引导信息
#[derive(Clone, Serialize)]
pub struct DeviceProfile {
    /// BLE 广播名匹配模式（包含匹配，大小写不敏感）
    pub name_patterns: Vec<String>,
    /// 显示名称
    pub display_name: String,
    /// 预期广播间隔（毫秒）
    pub expected_interval_ms: u32,
    /// 是否需要在手机 App 中开启心率广播
    pub needs_app_config: bool,
    /// FTUE 设置引导步骤
    pub setup_steps: Vec<String>,
    /// 已知问题
    pub known_issues: Vec<String>,
    /// 品牌（中文）
    #[serde(default)]
    pub brand: String,
    /// 类型：手环 / 手表 / 运动手表 / 胸带 / 实验性
    #[serde(default)]
    pub category: String,
    /// 配套手机 App 名称（无则空）
    #[serde(default)]
    pub official_app: String,
    /// 连不上时的排查建议
    #[serde(default)]
    pub troubleshooting: Vec<String>,
}

/// 所有已知设备 Profile
///
/// 排列顺序注意：同品牌中更具体的 pattern（如胸带 "Polar H10"）
/// 放在更通用的 pattern（如手表 "Polar Vantage"）之前，
/// 确保 `match_profile` 的 first-match-wins 语义正确。
pub fn all_profiles() -> Vec<DeviceProfile> {
    vec![
        // ── 国产手环/手表（P0）──
        DeviceProfile {
            name_patterns: vec![
                "Mi Band".into(),
                "Xiaomi Band".into(),
                "Xiaomi Smart Band".into(),
                "Redmi Band".into(),
                "M2105B".into(),
                "M2216B".into(),
                "M2311B".into(),
                "M2345B".into(),
            ],
            display_name: "小米手环".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「小米运动健康」App（Mi Fitness）".into(),
                "点击底部「设备」标签，选择你的手环".into(),
                "进入「心率」或「健康监测」设置".into(),
                "找到并开启「心率广播」（部分固件在 设备设置 → 心率 中）".into(),
                "确保手环蓝牙已开启，且手机 App 未独占 BLE 连接（可临时关闭 App 后台）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "部分固件版本需重启手环后才能开启广播".into(),
                "小米手环 7 及更早型号可能不支持 BLE 心率广播".into(),
                "开启广播后手机 App 可能断开与手环的连接，属正常现象".into(),
            ],
            brand: "小米".into(),
            category: "手环".into(),
            official_app: "小米运动健康（Mi Fitness）".into(),
            troubleshooting: vec![
                "扫描不到设备：重启手环蓝牙，或重启手环".into(),
                "连接后无数据：确认已在 App 中开启「心率广播」".into(),
                "手机 App 独占连接：关闭小米运动健康 App 后台，再扫描".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Xiaomi Watch".into(),
                "Redmi Watch".into(),
                "M2329W1".into(),
                "M2424W1".into(),
            ],
            display_name: "小米/红米手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「小米运动健康」App（Mi Fitness）".into(),
                "点击底部「设备」标签，选择你的手表".into(),
                "进入「心率」或「健康监测」设置".into(),
                "找到并开启「心率广播」".into(),
                "确保手表蓝牙已开启，且手机 App 未独占 BLE 连接".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "小米手表 S3/S4、Redmi Watch 系列均需在 App 中开启广播".into(),
                "开启广播后手机 App 可能断开连接，属正常现象".into(),
            ],
            brand: "小米".into(),
            category: "手表".into(),
            official_app: "小米运动健康（Mi Fitness）".into(),
            troubleshooting: vec![
                "扫描不到设备：重启手表蓝牙，或重启手表".into(),
                "连接后无数据：确认已在 App 中开启「心率广播」".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "HUAWEI Band".into(),
                "HUAWEI WATCH".into(),
                "HUAWEI GT".into(),
                "HUAWEI FIT".into(),
                "HUAWEI Ultimate".into(),
            ],
            display_name: "华为手环/手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「华为运动健康」App（Huawei Health）".into(),
                "点击底部「设备」标签，选择你的手环/手表".into(),
                "进入「心率」设置".into(),
                "开启「心率实时广播」（部分型号在 设备设置 → 心率 → 实时广播 中）".into(),
                "部分型号需在手表端 设置 → 蓝牙 → 允许被其他设备发现".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "GT 系列手表广播间隔可能为 2s，属正常现象".into(),
                "需确保手环/手表未通过手机 App 独占 BLE 连接".into(),
                "WATCH 4 / Ultimate 系列需在 App 中确认开启实时广播".into(),
            ],
            brand: "华为".into(),
            category: "手环".into(),
            official_app: "华为运动健康（Huawei Health）".into(),
            troubleshooting: vec![
                "扫描不到设备：在手表端开启「允许被其他设备发现」".into(),
                "连接后无数据：确认已在 App 中开启「心率实时广播」".into(),
                "手机 App 独占连接：关闭华为运动健康 App 后台，再扫描".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Honor Band".into(),
                "Honor MagicWatch".into(),
                "Honor Watch".into(),
                "HONOR Band".into(),
                "HONOR MagicWatch".into(),
                "HONOR Watch".into(),
            ],
            display_name: "荣耀手环/手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「荣耀运动健康」App（Honor Health）".into(),
                "点击底部「设备」标签，选择你的手环/手表".into(),
                "进入「心率」设置".into(),
                "开启「心率广播」或「心率实时广播」".into(),
                "确保手环/手表蓝牙已开启，且手机 App 未独占 BLE 连接".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "荣耀设备使用独立的「荣耀运动健康」App（非华为运动健康）".into(),
                "MagicWatch / Watch GS 系列广播间隔可能为 2s".into(),
            ],
            brand: "荣耀".into(),
            category: "手环".into(),
            official_app: "荣耀运动健康（Honor Health）".into(),
            troubleshooting: vec![
                "扫描不到设备：确认使用的是「荣耀运动健康」而非「华为运动健康」App".into(),
                "连接后无数据：确认已在 App 中开启心率广播".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Amazfit".into(),
                "GTR".into(),
                "GTS".into(),
                "T-Rex".into(),
                "Cheetah".into(),
                "Falcon".into(),
                "Balance".into(),
                "Active".into(),
                "Helio".into(),
            ],
            display_name: "Amazfit 华米手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "确保手表蓝牙已开启（多数 Amazfit 手表默认开启）".into(),
                "若扫描不到：在手表端进入 设置 → 心率 → 查找「心率广播 / Broadcast HR」并开启（部分型号需要）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "多数 Amazfit 手表即连即用，无需额外配置".into(),
                "部分较新固件可能需要在手表设置中手动开启心率广播".into(),
                "GTR/GTS 4 及以上、T-Rex 2/Ultra、Cheetah、Falcon、Balance、Active 均支持".into(),
            ],
            brand: "华米".into(),
            category: "运动手表".into(),
            official_app: "Zepp".into(),
            troubleshooting: vec![
                "扫描不到设备：在手表设置中查找「心率广播」并开启".into(),
                "连接后无数据：重启手表蓝牙后重试".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "OPPO Watch".into(),
                "OPPO Band".into(),
            ],
            display_name: "OPPO 手表/手环".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「OPPO 健康」App（HeyTap Health）".into(),
                "进入「设备」→ 选择你的手表/手环".into(),
                "在设置中查找「心率广播」或「实时心率共享」并开启".into(),
                "确保手表蓝牙已开启，且手机 App 未独占 BLE 连接".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "OPPO Watch 系列需在 OPPO 健康 App 中开启心率广播".into(),
                "具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」".into(),
            ],
            brand: "OPPO".into(),
            category: "手表".into(),
            official_app: "OPPO 健康（HeyTap Health）".into(),
            troubleshooting: vec![
                "扫描不到设备：重启手表蓝牙后重试".into(),
                "连接后无数据：确认已在 App 中开启心率广播".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "vivo Watch".into(),
                "vivo Band".into(),
            ],
            display_name: "vivo 手表/手环".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「vivo 健康」App".into(),
                "进入「设备」→ 选择你的手表/手环".into(),
                "在设置中查找「心率广播」或「实时心率」并开启".into(),
                "确保手表蓝牙已开启，且手机 App 未独占 BLE 连接".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "vivo Watch 系列需在 vivo 健康 App 中开启心率广播".into(),
                "具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」".into(),
            ],
            brand: "vivo".into(),
            category: "手表".into(),
            official_app: "vivo 健康".into(),
            troubleshooting: vec![
                "扫描不到设备：重启手表蓝牙后重试".into(),
                "连接后无数据：确认已在 App 中开启心率广播".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "OnePlus Watch".into(),
                "OnePlus Band".into(),
            ],
            display_name: "一加手表/手环".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「OnePlus Health」App（一加健康）".into(),
                "进入「设备」→ 选择你的手表".into(),
                "在设置中查找「心率广播」或「实时心率共享」并开启".into(),
                "确保手表蓝牙已开启，且手机 App 未独占 BLE 连接".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "OnePlus Watch 系列需在 OnePlus Health App 中开启心率广播".into(),
                "具体菜单路径因固件版本而异，若找不到请在手表端设置中搜索「心率」".into(),
            ],
            brand: "一加".into(),
            category: "手表".into(),
            official_app: "OnePlus Health（一加健康）".into(),
            troubleshooting: vec![
                "扫描不到设备：重启手表蓝牙后重试".into(),
                "连接后无数据：确认已在 App 中开启心率广播".into(),
            ],
        },
        // ── 国际运动手表（P1）──
        DeviceProfile {
            name_patterns: vec![
                "Forerunner".into(),
                "Fenix".into(),
                "Epix".into(),
                "Venu".into(),
                "Vivoactive".into(),
                "Instinct".into(),
                "Garmin".into(),
                "Enduro".into(),
                "Tactix".into(),
            ],
            display_name: "Garmin 佳明运动表".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "在手表上进入「设置」（长按左上键或从表盘上滑）".into(),
                "选择「传感器与配件」（Sensors & Accessories）".into(),
                "选择「心率」（Heart Rate）".into(),
                "开启「广播心率」（Broadcast Heart Rate）".into(),
                "（部分型号路径：设置 → 活动与 App → 心率 → 广播心率）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "广播心率功能需手表端手动开启，默认关闭".into(),
                "开启广播后手表电量消耗增加".into(),
                "Forerunner 245 及以上、Fenix 6 及以上、Venu 2 及以上、Epix、Instinct 2 及以上均支持".into(),
            ],
            brand: "Garmin".into(),
            category: "运动手表".into(),
            official_app: "Garmin Connect".into(),
            troubleshooting: vec![
                "扫描不到设备：确认已在手表端开启「广播心率」".into(),
                "连接后无数据：关闭 Garmin Connect App 后台，避免独占连接".into(),
                "广播间隔偏大：属正常现象，Garmin 广播间隔约 1s".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "COROS".into(),
                "PACE".into(),
                "VERTIX".into(),
                "APEX".into(),
            ],
            display_name: "COROS 高驰运动表".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "在手表上进入「设置」（从表盘长按返回键或上滑进入）".into(),
                "选择「更多」（More）或「系统设置」".into(),
                "找到「心率广播」（HR Broadcast）并开启".into(),
                "（部分型号路径：设置 → 更多 → 心率广播）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "PACE 2/3、VERTIX 2、APEX 2/APEX 2 Pro 均支持心率广播".into(),
                "广播功能需手表端手动开启".into(),
            ],
            brand: "COROS".into(),
            category: "运动手表".into(),
            official_app: "COROS".into(),
            troubleshooting: vec![
                "扫描不到设备：确认已在手表端开启「心率广播」".into(),
                "连接后无数据：重启手表蓝牙后重试".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Suunto".into(),
            ],
            display_name: "Suunto 颂拓运动表".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "在手表上进入「设置」（长按中间键或从表盘上滑）".into(),
                "选择「连接」（Connectivity）或「一般设置」（General）".into(),
                "找到「心率广播」（HR broadcast）并开启".into(),
                "（部分型号路径：设置 → 连接 → 心率广播）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "Suunto Race、Vertical、9 Peak Pro 等较新型号支持心率广播".into(),
                "广播功能需手表端手动开启，默认关闭".into(),
            ],
            brand: "Suunto".into(),
            category: "运动手表".into(),
            official_app: "Suunto".into(),
            troubleshooting: vec![
                "扫描不到设备：确认已在手表端开启「心率广播」".into(),
                "连接后无数据：重启手表蓝牙后重试".into(),
            ],
        },
        // ── Polar 胸带（放在 Polar 手表之前，确保 "Polar H10" 优先匹配）──
        DeviceProfile {
            name_patterns: vec![
                "Polar H10".into(),
                "Polar H9".into(),
                "Polar H7".into(),
            ],
            display_name: "Polar 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带，确保电极片贴合皮肤（可用水微微润湿电极片）".into(),
                "等待约 5 秒，胸带自动开机并开始广播".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "Polar H10/H9 同时支持 BLE 和 ANT+，PulseCast 使用 BLE".into(),
                "胸带需贴合皮肤才能检测心率，悬空无数据".into(),
            ],
            brand: "Polar".into(),
            category: "胸带".into(),
            official_app: String::new(),
            troubleshooting: vec![
                "扫描不到设备：确认胸带已佩戴且电极片贴合皮肤".into(),
                "连接后无数据：润湿电极片，确保胸带紧贴胸部".into(),
                "电量不足：更换 CR2025 纽扣电池".into(),
            ],
        },
        // ── Polar 手表 ──
        DeviceProfile {
            name_patterns: vec![
                "Polar Vantage".into(),
                "Polar Grit".into(),
                "Polar Pacer".into(),
                "Polar Ignite".into(),
                "Polar Unite".into(),
            ],
            display_name: "Polar 运动手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "在手表上进入「设置」（长按返回键或从表盘进入设置）".into(),
                "选择「一般设置」（General settings）".into(),
                "找到「心率广播」（HR broadcast）并开启".into(),
                "（部分型号路径：设置 → 一般 → 心率广播）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "Vantage V2/M2、Grit X/Pro、Pacer/Pacer Pro、Ignite 3 均支持".into(),
                "广播功能需手表端手动开启".into(),
            ],
            brand: "Polar".into(),
            category: "运动手表".into(),
            official_app: "Polar Flow".into(),
            troubleshooting: vec![
                "扫描不到设备：确认已在手表端开启「心率广播」".into(),
                "连接后无数据：关闭 Polar Flow App 后台，避免独占连接".into(),
            ],
        },
        // ── 胸带 ──
        DeviceProfile {
            name_patterns: vec![
                "TICKR".into(),
                "Wahoo".into(),
            ],
            display_name: "Wahoo TICKR 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带，确保电极片贴合皮肤".into(),
                "胸带自动开机并开始 BLE 广播".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "TICKR / TICKR X / TICKR Fit 均支持 BLE 心率广播".into(),
                "TICKR X 同时支持 BLE 和 ANT+".into(),
            ],
            brand: "Wahoo".into(),
            category: "胸带".into(),
            official_app: String::new(),
            troubleshooting: vec![
                "扫描不到设备：确认胸带已佩戴且电极片贴合皮肤".into(),
                "连接后无数据：润湿电极片，确保胸带紧贴".into(),
                "电量不足：更换 CR2032 纽扣电池".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "HRM-Pro".into(),
                "HRM-Dual".into(),
                "HRM-Fit".into(),
                "Garmin HRM".into(),
            ],
            display_name: "Garmin 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）".into(),
                "胸带自动开机并开始 BLE 广播".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "HRM-Pro / HRM-Pro Plus / HRM-Dual / HRM-Fit 均支持 BLE".into(),
                "HRM-Dual 同时支持 BLE 和 ANT+".into(),
            ],
            brand: "Garmin".into(),
            category: "胸带".into(),
            official_app: String::new(),
            troubleshooting: vec![
                "扫描不到设备：确认胸带已佩戴且电极片贴合皮肤".into(),
                "连接后无数据：润湿电极片，确保胸带紧贴".into(),
                "电量不足：更换 CR2032 纽扣电池".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Magene".into(),
                "H303".into(),
                "H64".into(),
                "L508".into(),
            ],
            display_name: "迈金 Magene 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）".into(),
                "胸带自动开机并开始 BLE 广播".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "Magene H303 / H64 / L508 均支持 BLE 心率广播".into(),
                "性价比高，国产胸带首选".into(),
            ],
            brand: "迈金".into(),
            category: "胸带".into(),
            official_app: String::new(),
            troubleshooting: vec![
                "扫描不到设备：确认胸带已佩戴且电极片贴合皮肤".into(),
                "连接后无数据：润湿电极片，确保胸带紧贴".into(),
                "电量不足：更换 CR2032 纽扣电池".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "Coospo".into(),
            ],
            display_name: "Coospo 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带，确保电极片贴合皮肤（可用水微微润湿）".into(),
                "胸带自动开机并开始 BLE 广播".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "Coospo H808S / H9Z 等型号支持 BLE 心率广播".into(),
            ],
            brand: "Coospo".into(),
            category: "胸带".into(),
            official_app: String::new(),
            troubleshooting: vec![
                "扫描不到设备：确认胸带已佩戴且电极片贴合皮肤".into(),
                "连接后无数据：润湿电极片，确保胸带紧贴".into(),
            ],
        },
        // ── 实验性 ──
        DeviceProfile {
            name_patterns: vec![
                "Apple Watch".into(),
            ],
            display_name: "Apple Watch（实验性）".into(),
            expected_interval_ms: 2000,
            needs_app_config: true,
            setup_steps: vec![
                "⚠ Apple Watch 原生不广播标准 BLE 心率，需第三方 App 中转".into(),
                "在 Apple Watch 上安装第三方心率广播 App（如 HeartCast、Heart Rate Broadcast 等）".into(),
                "打开该 App，按 App 内指引开启 BLE 心率广播".into(),
                "确保 iPhone 蓝牙已开启，Apple Watch 与 iPhone 已连接".into(),
                "回到本软件点击「扫描蓝牙设备」（广播名可能显示为第三方 App 名称）".into(),
            ],
            known_issues: vec![
                "原生不广播标准 BLE 心率，必须依赖第三方 App 中转".into(),
                "需第三方 App 中转，非官方支持".into(),
                "v0.9 不保证 Apple Watch 兼容性，体验可能不稳定".into(),
                "广播间隔通常为 2s 或更长，延迟较高".into(),
                "第三方 App 可能需要付费或订阅".into(),
            ],
            brand: "Apple".into(),
            category: "实验性".into(),
            official_app: "HeartCast（第三方）".into(),
            troubleshooting: vec![
                "扫描不到设备：确认第三方 App 已开启广播且在前台运行".into(),
                "连接后无数据：重启第三方 App，确认 Apple Watch 心率传感器正常工作".into(),
                "延迟高：Apple Watch 广播间隔较大，属平台限制".into(),
            ],
        },
    ]
}

/// 根据 BLE 广播名匹配设备 Profile（大小写不敏感）
pub fn match_profile(ble_name: &str) -> Option<DeviceProfile> {
    let lower = ble_name.to_lowercase();
    all_profiles()
        .into_iter()
        .find(|p| p.name_patterns.iter().any(|pat| lower.contains(&pat.to_lowercase())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_profiles_count() {
        let profiles = all_profiles();
        assert!(
            profiles.len() >= 15,
            "Expected >= 15 profiles, got {}",
            profiles.len()
        );
    }

    #[test]
    fn test_profile_fields_nonempty() {
        for p in all_profiles() {
            assert!(!p.display_name.is_empty(), "display_name must not be empty");
            assert!(
                !p.name_patterns.is_empty(),
                "name_patterns must not be empty for {}",
                p.display_name
            );
            assert!(
                !p.setup_steps.is_empty(),
                "setup_steps must not be empty for {}",
                p.display_name
            );
            assert!(!p.brand.is_empty(), "brand must not be empty for {}", p.display_name);
            assert!(
                !p.category.is_empty(),
                "category must not be empty for {}",
                p.display_name
            );
        }
    }

    #[test]
    fn test_match_xiaomi() {
        let profile = match_profile("Mi Band 8").expect("should match Mi Band 8");
        assert_eq!(profile.brand, "小米");
    }

    #[test]
    fn test_match_huawei() {
        let profile = match_profile("HUAWEI WATCH GT4").expect("should match HUAWEI WATCH GT4");
        assert_eq!(profile.brand, "华为");
    }

    #[test]
    fn test_match_garmin() {
        let profile = match_profile("Forerunner 265").expect("should match Forerunner 265");
        assert_eq!(profile.brand, "Garmin");
        assert_eq!(profile.category, "运动手表");
    }

    #[test]
    fn test_match_polar_h10() {
        let profile = match_profile("Polar H10").expect("should match Polar H10");
        assert_eq!(profile.category, "胸带");
        assert_eq!(profile.brand, "Polar");
    }

    #[test]
    fn test_match_coros() {
        let profile = match_profile("COROS PACE 3").expect("should match COROS PACE 3");
        assert_eq!(profile.brand, "COROS");
    }

    #[test]
    fn test_match_apple_watch() {
        let profile = match_profile("Apple Watch").expect("should match Apple Watch");
        assert_eq!(profile.category, "实验性");
        assert!(profile.needs_app_config);
    }

    #[test]
    fn test_match_unknown() {
        assert!(
            match_profile("RandomXYZ").is_none(),
            "Unknown device should not match"
        );
    }

    #[test]
    fn test_match_honor() {
        let profile = match_profile("Honor Band 9").expect("should match Honor Band 9");
        assert_eq!(profile.brand, "荣耀");
    }

    #[test]
    fn test_match_magene() {
        let profile = match_profile("Magene H303").expect("should match Magene H303");
        assert_eq!(profile.brand, "迈金");
        assert_eq!(profile.category, "胸带");
    }

    #[test]
    fn test_match_amazfit() {
        let profile = match_profile("Amazfit GTR 4").expect("should match Amazfit GTR 4");
        assert_eq!(profile.brand, "华米");
    }

    #[test]
    fn test_match_wahoo() {
        let profile = match_profile("TICKR X").expect("should match TICKR X");
        assert_eq!(profile.brand, "Wahoo");
        assert_eq!(profile.category, "胸带");
    }

    #[test]
    fn test_match_garmin_hrm() {
        let profile = match_profile("HRM-Pro Plus").expect("should match HRM-Pro Plus");
        assert_eq!(profile.category, "胸带");
        assert_eq!(profile.brand, "Garmin");
    }

    #[test]
    fn test_match_suunto() {
        let profile = match_profile("Suunto Race").expect("should match Suunto Race");
        assert_eq!(profile.brand, "Suunto");
    }

    #[test]
    fn test_apple_watch_known_issues() {
        let profile = match_profile("Apple Watch").expect("should match Apple Watch");
        let issues = profile.known_issues.join(" ");
        assert!(
            issues.contains("原生不广播标准BLE心率") || issues.contains("原生不广播标准 BLE 心率"),
            "Apple Watch known_issues must mention native BLE limitation"
        );
        assert!(
            issues.contains("第三方App") || issues.contains("第三方 App"),
            "Apple Watch known_issues must mention third-party app"
        );
        assert!(
            issues.contains("非官方支持"),
            "Apple Watch known_issues must state unofficial support"
        );
    }
}
