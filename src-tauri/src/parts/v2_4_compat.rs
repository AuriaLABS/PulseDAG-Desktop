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

    #[test]
    fn v2_4_observability_dtos_accept_extended_rpc_shapes() {
        let status: NodeStatusData = serde_json::from_value(serde_json::json!({
            "rpc_response_degraded": false,
            "rpc_response_stale": false,
            "rpc_response_degraded_reason": null,
            "network_id": "private-testnet-v2.4.0",
            "service": "pulsedagd",
            "version": "2.4.0",
            "chain_id": "pulsedag-private-v2.4.0",
            "best_height": 42,
            "block_count": 43,
            "selected_tip": "a".repeat(64),
            "selected_height": 42,
            "consensus_mode": "legacy",
            "tip_count": 1,
            "orphan_count": 0,
            "mempool_size": 2,
            "snapshot_height": 40,
            "persisted_block_count": 43,
            "p2p_mode": "libp2p-real",
            "peer_count": 4,
            "sync_state": "synced",
            "storage_backend": "rocksdb",
            "ordering_version": "legacy",
            "connected_peers_are_real_network": true,
            "contracts_enabled": false
        }))
        .expect("v2.4 status subset must remain compatible");
        assert_eq!(status.version, "2.4.0");
        assert_eq!(status.chain_id, V2_4_PRIVATE_CHAIN_ID);

        let sync: SyncStatusData = serde_json::from_value(serde_json::json!({
            "rpc_response_degraded": false,
            "rpc_response_stale": false,
            "consistency_ok": true,
            "consistency_issue_count": 0,
            "lag_blocks": 0,
            "sync_state": "synced",
            "network_selected_height_gap": 0,
            "storage_replay_gap": 0,
            "live_sync_error_active": 0,
            "p2p_ready_for_private_rehearsal": true,
            "readiness_reasons": [],
            "canonical_sync_state_generation": 7,
            "catchup_stage": "aligned"
        }))
        .expect("v2.4 sync subset must remain compatible");
        assert!(sync.consistency_ok);

        let mempool: MempoolData = serde_json::from_value(serde_json::json!({
            "transaction_count": 2,
            "orphan_transaction_count": 0,
            "orphan_limit": 1024,
            "spent_outpoints_count": 3,
            "txids": ["b".repeat(64)],
            "orphaned_total": 5,
            "orphan_promoted_total": 4
        }))
        .expect("v2.4 mempool subset must remain compatible");
        assert_eq!(mempool.transaction_count, 2);

        let pow: PowHealthData = serde_json::from_value(serde_json::json!({
            "status": "ok",
            "snapshot_count": 8,
            "latest_suggested_difficulty": 12,
            "latest_avg_block_interval_secs": 60,
            "alerts": []
        }))
        .expect("v2.4 PoW health must remain compatible");
        assert_eq!(pow.latest_suggested_difficulty, 12.0);

        let blocks: RecentBlocksData = serde_json::from_value(serde_json::json!({
            "count": 1,
            "total": 1,
            "limit": 20,
            "offset": 0,
            "has_more": false,
            "blocks": [{
                "hash": "c".repeat(64),
                "height": 42,
                "blue_score": 42,
                "tx_count": 1,
                "timestamp": 1_700_000_000,
                "parent_count": 1
            }]
        }))
        .expect("v2.4 recent block subset must remain compatible");
        assert_eq!(blocks.blocks.len(), 1);
    }

    #[test]
    fn v2_4_observability_paths_remain_read_only_and_exact() {
        assert_eq!(OBSERVABILITY_STATUS_PATH, "/api/v1/status");
        assert_eq!(OBSERVABILITY_SYNC_PATH, "/api/v1/sync/status");
        assert_eq!(OBSERVABILITY_MEMPOOL_PATH, "/api/v1/mempool");
        assert_eq!(OBSERVABILITY_POW_PATH, "/api/v1/pow/health");
        assert_eq!(OBSERVABILITY_BLOCKS_PATH, "/api/v1/blocks/recent?limit=20");
        assert!(approved_observability_path(OBSERVABILITY_STATUS_PATH));
        assert!(!approved_observability_path("/api/v1/tx/submit"));
        assert!(!approved_observability_path("/api/v1/mining/submit"));
    }
}
