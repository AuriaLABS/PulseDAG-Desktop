#[cfg(test)]
mod v2_4_task31_tests {
    use super::*;

    const TASK31_PRE_FREEZE_TECHNICAL_CANDIDATE: &str =
        "265bf83e8f58e1c1cedc3a6467f334d60d9ef283";

    fn node_proof(source_commit: &str, archive_sha256: &str) -> TrustedBinaryProvenance {
        let archive_name = "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz";
        let layout = v2_4_final_archive_layout(archive_name, V2_4CandidateBinaryKind::Node)
            .expect("final node layout");
        TrustedBinaryProvenance {
            executable_path: "/tmp/pulsedagd".into(),
            binary_sha256: "a".repeat(64),
            binary_size_bytes: 4,
            archive_name: archive_name.into(),
            archive_sha256: archive_sha256.into(),
            release_tag: V2_4_FINAL_RELEASE_TAG.into(),
            source_commit: source_commit.into(),
            target: layout.target,
            embedded_path: layout.binary_path.display().to_string(),
            linked_at_ms: 1,
        }
    }

    #[test]
    fn v2_4_task31_final_release_identity_is_exact() {
        assert_eq!(V2_4_FINAL_RELEASE_TAG, "v2.4.0");
        assert_eq!(
            V2_4_FINAL_RELEASE_SOURCE_COMMIT,
            "876b48826a3875b729888edb88e2b0eea15bb717"
        );
        assert_eq!(
            v2_4_private_identity(),
            ("private-testnet-v2.4.0", "pulsedag-private-v2.4.0")
        );
    }

    #[test]
    fn v2_4_task31_pre_freeze_candidate_stays_rejected() {
        let archive = "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz";
        let digest = v2_4_final_archive_digest(archive).expect("frozen archive digest");
        assert!(!is_final_v2_4_release_provenance(&node_proof(
            TASK31_PRE_FREEZE_TECHNICAL_CANDIDATE,
            digest,
        )));
    }

    #[test]
    fn v2_4_task31_final_release_requires_frozen_archive_digest() {
        let archive = "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz";
        let digest = v2_4_final_archive_digest(archive).expect("frozen archive digest");
        assert!(is_final_v2_4_release_provenance(&node_proof(
            V2_4_FINAL_RELEASE_SOURCE_COMMIT,
            digest,
        )));
        assert!(!is_final_v2_4_release_provenance(&node_proof(
            V2_4_FINAL_RELEASE_SOURCE_COMMIT,
            &"0".repeat(64),
        )));
    }
}
