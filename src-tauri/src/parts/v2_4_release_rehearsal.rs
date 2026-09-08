#[cfg(test)]
mod v2_4_release_rehearsal_tests {
    use super::*;
    use std::env;

    fn required_path(name: &str) -> PathBuf {
        env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("{name} must point to a final v2.4.0 release artifact"))
    }

    fn expected_digest(name: &str) -> String {
        env::var(name).unwrap_or_else(|_| panic!("{name} must contain the frozen archive SHA-256"))
    }

    #[test]
    #[ignore = "requires the published PulseDAG v2.4.0 node archive and extracted binary"]
    fn v2_4_final_node_release_rehearsal() {
        let archive_path = required_path("PULSEDAG_V24_NODE_ARCHIVE");
        let binary_path = required_path("PULSEDAG_V24_NODE_BINARY");
        let expected_archive_sha256 = expected_digest("PULSEDAG_V24_NODE_ARCHIVE_SHA256");

        let inspected = inspect_v2_4_final_archive_path(
            &archive_path,
            V2_4CandidateBinaryKind::Node,
        )
        .expect("inspect published v2.4.0 node archive");
        let selected = validate_binary_path(&binary_path)
            .expect("validate extracted published v2.4.0 pulsedagd binary");

        assert_eq!(
            inspected.archive_sha256.to_ascii_lowercase(),
            expected_archive_sha256.to_ascii_lowercase(),
            "Rust archive inspection must match the workflow's frozen digest"
        );
        assert_eq!(
            inspected.embedded_binary_sha256.to_ascii_lowercase(),
            selected.sha256.to_ascii_lowercase(),
            "extracted pulsedagd must be byte-for-byte identical to the archive member"
        );
        assert_eq!(inspected.embedded_binary_size_bytes, selected.size_bytes);

        let proof = TrustedBinaryProvenance {
            executable_path: selected.path,
            binary_sha256: selected.sha256,
            binary_size_bytes: selected.size_bytes,
            archive_name: inspected.archive_name,
            archive_sha256: inspected.archive_sha256,
            release_tag: V2_4_FINAL_RELEASE_TAG.into(),
            source_commit: V2_4_FINAL_RELEASE_SOURCE_COMMIT.into(),
            target: inspected.target,
            embedded_path: inspected.embedded_path,
            linked_at_ms: unix_time_ms(),
        };
        assert!(
            is_final_v2_4_release_provenance(&proof),
            "published node archive must satisfy the final Task31 provenance gate"
        );
    }

    #[test]
    #[ignore = "requires the published PulseDAG v2.4.0 miner archive and extracted binary"]
    fn v2_4_final_miner_release_rehearsal() {
        let archive_path = required_path("PULSEDAG_V24_MINER_ARCHIVE");
        let binary_path = required_path("PULSEDAG_V24_MINER_BINARY");
        let expected_archive_sha256 = expected_digest("PULSEDAG_V24_MINER_ARCHIVE_SHA256");

        let inspected = inspect_v2_4_final_archive_path(
            &archive_path,
            V2_4CandidateBinaryKind::Miner,
        )
        .expect("inspect published v2.4.0 miner archive");
        let selected = validate_miner_binary_path(&binary_path)
            .expect("validate extracted published v2.4.0 pulsedag-miner binary");

        assert_eq!(
            inspected.archive_sha256.to_ascii_lowercase(),
            expected_archive_sha256.to_ascii_lowercase(),
            "Rust miner archive inspection must match the workflow's frozen digest"
        );
        assert_eq!(
            inspected.embedded_binary_sha256.to_ascii_lowercase(),
            selected.sha256.to_ascii_lowercase(),
            "extracted pulsedag-miner must be byte-for-byte identical to the archive member"
        );
        assert_eq!(inspected.embedded_binary_size_bytes, selected.size_bytes);

        let proof = TrustedBinaryProvenance {
            executable_path: selected.path,
            binary_sha256: selected.sha256,
            binary_size_bytes: selected.size_bytes,
            archive_name: inspected.archive_name,
            archive_sha256: inspected.archive_sha256,
            release_tag: V2_4_FINAL_RELEASE_TAG.into(),
            source_commit: V2_4_FINAL_RELEASE_SOURCE_COMMIT.into(),
            target: inspected.target,
            embedded_path: inspected.embedded_path,
            linked_at_ms: unix_time_ms(),
        };
        assert!(
            is_final_v2_4_release_provenance(&proof),
            "published miner archive must satisfy the final Task31 provenance gate"
        );
    }
}
