#[cfg(test)]
mod v2_4_task31_tests {
    use super::*;

    const TASK31_CURRENT_TECHNICAL_CANDIDATE: &str =
        "265bf83e8f58e1c1cedc3a6467f334d60d9ef283";

    fn proof(source_commit: &str) -> TrustedBinaryProvenance {
        TrustedBinaryProvenance {
            executable_path: "/tmp/pulsedagd".into(),
            binary_sha256: "a".repeat(64),
            binary_size_bytes: 4,
            archive_name: "pulsedagd-v2.4.0-test.tar.gz".into(),
            archive_sha256: "b".repeat(64),
            release_tag: V2_4_CANDIDATE_TAG.into(),
            source_commit: source_commit.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            embedded_path: "pulsedagd-v2.4.0-test/pulsedagd".into(),
            linked_at_ms: 1,
        }
    }

    #[test]
    fn v2_4_task31_technical_candidate_is_not_release_provenance() {
        assert!(V2_4_FINAL_RELEASE_COMMIT.is_none());
        assert!(!is_approved_v2_4_provenance(&proof(
            TASK31_CURRENT_TECHNICAL_CANDIDATE
        )));
    }
}
