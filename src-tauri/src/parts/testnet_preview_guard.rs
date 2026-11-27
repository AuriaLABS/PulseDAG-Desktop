const TESTNET_PREVIEW_INSTALL_ID: &str = "io.aurialabs.pulsedag.desktop.testnet";

/// A testnet installer is intentionally a non-operational preview until
/// Core #781 grants GO_PUBLIC_TESTNET and the exact native testnet release is
/// separately implemented and reviewed. Neither v2.4 nor provisional v3
/// miners/nodes may be started from this channel.
fn testnet_preview_launch_guard(
    package_version: &str,
    app_identifier: &str,
) -> Result<(), String> {
    if package_version.contains("-testnet.")
        || app_identifier == TESTNET_PREVIEW_INSTALL_ID
    {
        return Err(
            "Desktop Testnet preview is NO-GO: native node/miner launches are disabled until an approved, identity-bound Core public-testnet release is integrated."
                .into(),
        );
    }
    Ok(())
}

fn reject_testnet_preview_launch(app: &AppHandle) -> Result<(), String> {
    testnet_preview_launch_guard(env!("CARGO_PKG_VERSION"), &app.config().identifier)
}

#[cfg(test)]
mod testnet_preview_guard_tests {
    use super::*;

    #[test]
    fn testnet_preview_guard_preserves_historical_stable_launch_path() {
        assert!(testnet_preview_launch_guard(
            "0.1.0",
            "io.aurialabs.pulsedag.desktop"
        ).is_ok());
    }

    #[test]
    fn testnet_preview_guard_rejects_testnet_installer_even_if_package_version_is_stable() {
        let error = testnet_preview_launch_guard(
            "0.1.0",
            TESTNET_PREVIEW_INSTALL_ID,
        ).unwrap_err();
        assert!(error.contains("NO-GO"));
    }

    #[test]
    fn testnet_preview_guard_rejects_testnet_package_even_if_installer_id_is_stable() {
        assert!(testnet_preview_launch_guard(
            "0.1.0-testnet.1",
            "io.aurialabs.pulsedag.desktop"
        ).is_err());
        assert!(testnet_preview_launch_guard(
            "0.1.0-testnet.2",
            TESTNET_PREVIEW_INSTALL_ID
        ).is_err());
    }
}
