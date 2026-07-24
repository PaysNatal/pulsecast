//! 设备 Profile —— 为已知 BLE 心率设备提供适配引导
//!
//! 每种设备有不同的广播行为、配置要求和已知问题。
//! FTUE 根据用户选择的设备类型展示对应的设置步骤，
//! BLE 扫描结果按 Profile 分组显示。

use serde::Serialize;

/// 设备 Profile：描述一种已知 BLE 心率设备的特征和引导信息
#[derive(Clone, Serialize)]
pub struct DeviceProfile {
    /// BLE 广播名匹配模式（前缀匹配）
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
}

/// 所有已知设备 Profile
pub fn all_profiles() -> Vec<DeviceProfile> {
    vec![
        DeviceProfile {
            name_patterns: vec![
                "Mi Band".into(),
                "Xiaomi Band".into(),
                "M2105B".into(),
                "M2216B".into(),
                "M2311B".into(),
            ],
            display_name: "小米手环".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「小米运动健康」App".into(),
                "进入 设备 → 心率 → 开启「心率广播」".into(),
                "确保手环蓝牙已开启且未连接手机（或手机允许共享连接）".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "部分固件版本需重启手环后才能开启广播".into(),
                "小米手环 8 以下型号可能不支持 BLE 心率广播".into(),
            ],
        },
        DeviceProfile {
            name_patterns: vec![
                "HUAWEI".into(),
                "Honor Band".into(),
                "GT-".into(),
                "WATCH".into(),
            ],
            display_name: "华为手环/手表".into(),
            expected_interval_ms: 1000,
            needs_app_config: true,
            setup_steps: vec![
                "打开手机「华为运动健康」App".into(),
                "进入 设备 → 心率 → 开启「心率实时广播」".into(),
                "部分型号需在 设置 → 蓝牙 → 允许被其他设备发现".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![
                "GT 系列手表广播间隔可能为 2s，属正常现象".into(),
                "需确保手环未通过手机 App 独占 BLE 连接".into(),
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
            ],
            display_name: "Amazfit".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "确保手表蓝牙已开启".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![],
        },
        DeviceProfile {
            name_patterns: vec!["Polar".into()],
            display_name: "Polar 心率带".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带并湿润电极片".into(),
                "等待 5 秒自动开机".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![],
        },
        DeviceProfile {
            name_patterns: vec!["TICKR".into(), "Wahoo".into()],
            display_name: "Wahoo TICKR".into(),
            expected_interval_ms: 1000,
            needs_app_config: false,
            setup_steps: vec![
                "佩戴胸带".into(),
                "回到本软件点击「扫描蓝牙设备」".into(),
            ],
            known_issues: vec![],
        },
    ]
}

/// 根据 BLE 广播名匹配设备 Profile
pub fn match_profile(ble_name: &str) -> Option<DeviceProfile> {
    all_profiles()
        .into_iter()
        .find(|p| p.name_patterns.iter().any(|pat| ble_name.contains(pat.as_str())))
}
