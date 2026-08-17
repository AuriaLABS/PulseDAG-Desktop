pub(crate) const V2_4_CANDIDATE_TAG: &str = "v2.4.0";
pub(crate) const V2_4_CANDIDATE_COMMIT: &str =
    "995b7b200afc90f705eece6c37a16b7a4fc294ec";
pub(crate) const V2_4_RELEASE_API: &str =
    "https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.4.0";
pub(crate) const V2_4_PRIVATE_NETWORK_PROFILE: &str = "private-testnet-v2.4.0";
pub(crate) const V2_4_PRIVATE_CHAIN_ID: &str = "pulsedag-private-v2.4.0";
pub(crate) const V2_4_INSTALL_GUIDE: &str = "INSTALL_BINARIES_V2_4_0.md";

pub(crate) fn is_v2_4_candidate_archive_name(name: &str, binary: &str) -> bool {
    let prefix = format!("{binary}-{V2_4_CANDIDATE_TAG}-");
    name.starts_with(&prefix) && (name.ends_with(".tar.gz") || name.ends_with(".zip"))
}

pub(crate) fn v2_4_private_identity() -> (&'static str, &'static str) {
    (V2_4_PRIVATE_NETWORK_PROFILE, V2_4_PRIVATE_CHAIN_ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_4_candidate_identity_is_exact() {
        assert_eq!(V2_4_CANDIDATE_TAG, "v2.4.0");
        assert_eq!(
            V2_4_CANDIDATE_COMMIT,
            "995b7b200afc90f705eece6c37a16b7a4fc294ec"
        );
        assert_eq!(
            v2_4_private_identity(),
            ("private-testnet-v2.4.0", "pulsedag-private-v2.4.0")
        );
        assert_eq!(V2_4_INSTALL_GUIDE, "INSTALL_BINARIES_V2_4_0.md");
        assert!(V2_4_RELEASE_API.ends_with("/releases/tags/v2.4.0"));
    }

    #[test]
    fn v2_4_candidate_archive_names_are_bounded() {
        assert!(is_v2_4_candidate_archive_name(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip",
            "pulsedagd"
        ));
        assert!(is_v2_4_candidate_archive_name(
            "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
            "pulsedag-miner"
        ));
        assert!(!is_v2_4_candidate_archive_name(
            "pulsedagd-v2.3.0-x86_64-pc-windows-msvc.zip",
            "pulsedagd"
        ));
        assert!(!is_v2_4_candidate_archive_name(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip.exe",
            "pulsedagd"
        ));
    }
}
