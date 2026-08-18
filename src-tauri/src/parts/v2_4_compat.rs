pub(crate) const V2_4_CANDIDATE_TAG: &str = "v2.4.0";
pub(crate) const V2_4_CANDIDATE_COMMIT: &str =
    "995b7b200afc90f705eece6c37a16b7a4fc294ec";
pub(crate) const V2_4_RELEASE_API: &str =
    "https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.4.0";
pub(crate) const V2_4_PRIVATE_NETWORK_PROFILE: &str = "private-testnet-v2.4.0";
pub(crate) const V2_4_PRIVATE_CHAIN_ID: &str = "pulsedag-private-v2.4.0";
pub(crate) const V2_4_INSTALL_GUIDE: &str = "INSTALL_BINARIES_V2_4_0.md";
pub(crate) const V2_4_PRIVATE_STATE_MARKER: &str = ".pulsedag-desktop-v2.4-private";

pub(crate) fn is_v2_4_candidate_archive_name(name: &str, binary: &str) -> bool {
    let prefix = format!("{binary}-{V2_4_CANDIDATE_TAG}-");
    name.starts_with(&prefix) && (name.ends_with(".tar.gz") || name.ends_with(".zip"))
}

pub(crate) fn v2_4_private_identity() -> (&'static str, &'static str) {
    (V2_4_PRIVATE_NETWORK_PROFILE, V2_4_PRIVATE_CHAIN_ID)
}

pub(crate) fn is_approved_v2_4_provenance(proof: &TrustedBinaryProvenance) -> bool {
    proof.release_tag == V2_4_CANDIDATE_TAG && proof.source_commit == V2_4_CANDIDATE_COMMIT
}

pub(crate) fn v2_4_private_state_marker_contents() -> String {
    format!(
        "network_profile={V2_4_PRIVATE_NETWORK_PROFILE}\nchain_id={V2_4_PRIVATE_CHAIN_ID}\nrelease={V2_4_CANDIDATE_TAG}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(release_tag: &str, source_commit: &str) -> TrustedBinaryProvenance {
        TrustedBinaryProvenance {
            executable_path: "/tmp/pulsedagd".into(),
            binary_sha256: "a".repeat(64),
            binary_size_bytes: 4,
            archive_name: "pulsedagd-test.tar.gz".into(),
            archive_sha256: "b".repeat(64),
            release_tag: release_tag.into(),
            source_commit: source_commit.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            embedded_path: "pulsedagd-test/pulsedagd".into(),
            linked_at_ms: 1,
        }
    }

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
        assert_eq!(V2_4_PRIVATE_STATE_MARKER, ".pulsedag-desktop-v2.4-private");
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

    #[test]
    fn v2_4_private_identity_activates_only_for_exact_approved_provenance() {
        assert!(is_approved_v2_4_provenance(&proof(
            V2_4_CANDIDATE_TAG,
            V2_4_CANDIDATE_COMMIT
        )));
        assert!(!is_approved_v2_4_provenance(&proof(
            "v2.3.0",
            "7e43225f01ac05d15e5f1e3f1550d7850bf18cbc"
        )));
        assert!(!is_approved_v2_4_provenance(&proof(
            V2_4_CANDIDATE_TAG,
            "0000000000000000000000000000000000000000"
        )));
    }

    #[test]
    fn v2_4_private_state_marker_is_version_and_identity_bound() {
        let marker = v2_4_private_state_marker_contents();
        assert!(marker.contains("network_profile=private-testnet-v2.4.0"));
        assert!(marker.contains("chain_id=pulsedag-private-v2.4.0"));
        assert!(marker.contains("release=v2.4.0"));
        assert!(!marker.contains("v2.3.0"));
    }
}
