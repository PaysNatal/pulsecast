fn main() {
    // Windows：内嵌自定义应用清单，声明蓝牙设备能力（bluetooth），
    // 否则 btleplug 在 Windows 上无法扫描/连接 BLE 设备（静默回退模拟源）。
    // 该清单在非 Windows 目标上会被忽略，不影响 macOS / Linux 构建。
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");

    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
}
