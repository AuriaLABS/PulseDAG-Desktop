mod official_release_rehearsal_tests {
    use super::*;
    use std::env;

    fn required_path(name: &str) -> PathBuf {
        env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("{name} must point to an official release artifact"))
    }

    #[test]
    #[ignore = "requires the official PulseDAG v2.3.0 node archive and extracted binary"]
    fn provenance_official_release_rehearsal() {
        let archive_path = required_path("PULSEDAG_PROVENANCE_ARCHIVE");
        let binary_path = required_path("PULSEDAG_PROVENANCE_BINARY");
        let expected_archive_sha256 = env::var("PULSEDAG_PROVENANCE_ARCHIVE_SHA256")
            .expect("PULSEDAG_PROVENANCE_ARCHIVE_SHA256 must be set");

        let embedded = inspect_embedded_binary(&archive_path)
            .expect("inspect the official release archive");
        let selected = validate_binary_path(&binary_path)
            .expect("validate the extracted official pulsedagd binary");

        assert_eq!(
            embedded.target,
            supported_host_release_target().expect("supported native release target")
        );
        assert_eq!(
            embedded.archive_sha256.to_ascii_lowercase(),
            expected_archive_sha256.to_ascii_lowercase(),
            "the archive inspected by Rust must match the digest verified by the workflow"
        );
        assert_eq!(
            embedded.binary_sha256.to_ascii_lowercase(),
            selected.sha256.to_ascii_lowercase(),
            "the extracted binary must be byte-for-byte identical to the archive member"
        );
        assert_eq!(
            embedded.binary_size_bytes, selected.size_bytes,
            "the extracted binary size must match the archive member"
        );

        let supervisor = NodeSupervisor::default();
        assert!(
            verify_binary_provenance_for_launch(&supervisor, &selected, "private").is_err(),
            "private launch admission must fail before provenance is installed"
        );

        let archive_name = archive_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("official archive file name")
            .to_string();
        *supervisor
            .provenance
            .lock()
            .expect("provenance state") = Some(TrustedBinaryProvenance {
            executable_path: selected.path.clone(),
            binary_sha256: selected.sha256.clone(),
            binary_size_bytes: selected.size_bytes,
            archive_name,
            archive_sha256: embedded.archive_sha256,
            release_tag: APPROVED_RELEASE_TAG.into(),
            source_commit: APPROVED_RELEASE_COMMIT.into(),
            target: embedded.target,
            embedded_path: embedded.embedded_path,
            linked_at_ms: unix_time_ms(),
        });

        let admitted = verify_binary_provenance_for_launch(
            &supervisor,
            &selected,
            "private",
        )
        .expect("private launch admission with current provenance");
        assert!(admitted.is_some(), "private profile must accept the official binary");
    }

    #[test]
    #[ignore = "requires the official PulseDAG v2.3.0 miner archive and extracted binary"]
    fn miner_provenance_official_release_rehearsal() {
        let archive_path = required_path("PULSEDAG_MINER_PROVENANCE_ARCHIVE");
        let binary_path = required_path("PULSEDAG_MINER_PROVENANCE_BINARY");
        let expected_archive_sha256 = env::var("PULSEDAG_MINER_PROVENANCE_ARCHIVE_SHA256")
            .expect("PULSEDAG_MINER_PROVENANCE_ARCHIVE_SHA256 must be set");

        let embedded = inspect_embedded_miner_binary(&archive_path)
            .expect("inspect the official miner release archive");
        let selected = validate_miner_binary_path(&binary_path)
            .expect("validate the extracted official pulsedag-miner binary");

        assert_eq!(
            embedded.target,
            supported_host_release_target().expect("supported native release target")
        );
        assert_eq!(
            embedded.archive_sha256.to_ascii_lowercase(),
            expected_archive_sha256.to_ascii_lowercase(),
            "the miner archive inspected by Rust must match the workflow digest"
        );
        assert_eq!(
            embedded.binary_sha256.to_ascii_lowercase(),
            selected.sha256.to_ascii_lowercase(),
            "the extracted miner must be byte-for-byte identical to the archive member"
        );
        assert_eq!(
            embedded.binary_size_bytes, selected.size_bytes,
            "the extracted miner size must match the archive member"
        );

        let registry = MinerProvenanceRegistry::default();
        assert!(
            verify_miner_provenance_for_launch(&registry, &selected, "private").is_err(),
            "private mining admission must fail before miner provenance is installed"
        );

        let archive_name = archive_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("official miner archive file name")
            .to_string();
        *registry
            .provenance
            .lock()
            .expect("miner provenance state") = Some(TrustedBinaryProvenance {
            executable_path: selected.path.clone(),
            binary_sha256: selected.sha256.clone(),
            binary_size_bytes: selected.size_bytes,
            archive_name,
            archive_sha256: embedded.archive_sha256,
            release_tag: APPROVED_RELEASE_TAG.into(),
            source_commit: APPROVED_RELEASE_COMMIT.into(),
            target: embedded.target,
            embedded_path: embedded.embedded_path,
            linked_at_ms: unix_time_ms(),
        });

        let admitted = verify_miner_provenance_for_launch(&registry, &selected, "private")
            .expect("private miner launch admission with current provenance");
        assert!(admitted.is_some(), "private profile must accept the official miner");
    }
}
