fn main() {
    println!("cargo:rerun-if-env-changed=VITE_PULSEDAG_DESKTOP_CHANNEL");

    // Cargo and Vite must agree on the selected channel. Otherwise an
    // installer could carry the testnet identity with the stable UI (or vice versa).
    let package_testnet = env!("CARGO_PKG_VERSION").contains("-testnet.");
    let channel = std::env::var("VITE_PULSEDAG_DESKTOP_CHANNEL").unwrap_or_default();
    if !matches!(channel.as_str(), "" | "stable" | "testnet") {
        panic!("Unsupported VITE_PULSEDAG_DESKTOP_CHANNEL");
    }
    if package_testnet != (channel == "testnet") {
        panic!(
            "Desktop package/Vite channel mismatch: testnet Cargo prerelease and VITE_PULSEDAG_DESKTOP_CHANNEL=testnet must be selected together"
        );
    }

    tauri_build::build()
}
