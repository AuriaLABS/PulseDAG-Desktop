pub(crate) const V2_4_CANDIDATE_TAG: &str = "v2.4.0";
pub(crate) const V2_4_STALE_RELEASE_PREP_COMMIT: &str =
    "995b7b200afc90f705eece6c37a16b7a4fc294ec";
pub(crate) const V2_4_FINAL_RELEASE_COMMIT: Option<&str> = None;
pub(crate) const V2_4_RELEASE_API: &str =
    "https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.4.0";
pub(crate) const V2_4_PRIVATE_NETWORK_PROFILE: &str = "private-testnet-v2.4.0";
pub(crate) const V2_4_PRIVATE_CHAIN_ID: &str = "pulsedag-private-v2.4.0";
pub(crate) const V2_4_INSTALL_GUIDE: &str = "INSTALL_BINARIES_V2_4_0.md";
pub(crate) const V2_4_PRIVATE_STATE_MARKER: &str = ".pulsedag-desktop-v2.4-private";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum V2_4CandidateBinaryKind {
    Node,
    Miner,
}

impl V2_4CandidateBinaryKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "node" => Ok(Self::Node),
            "miner" => Ok(Self::Miner),
            _ => Err("v2.4 candidate binary kind must be node or miner.".into()),
        }
    }

    fn archive_prefix(self) -> &'static str {
        match self {
            Self::Node => "pulsedagd",
            Self::Miner => "pulsedag-miner",
        }
    }

    fn public_name(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Miner => "miner",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct V2_4CandidateArchiveInspection {
    archive_name: String,
    archive_sha256: String,
    archive_size_bytes: u64,
    binary_kind: String,
    release_tag: String,
    source_commit: String,
    target: String,
    embedded_path: String,
    embedded_binary_sha256: String,
    embedded_binary_size_bytes: u64,
    structurally_valid: bool,
    approved: bool,
    message: String,
}

pub(crate) fn is_v2_4_candidate_archive_name(name: &str, binary: &str) -> bool {
    let prefix = format!("{binary}-{V2_4_CANDIDATE_TAG}-");
    name.starts_with(&prefix) && (name.ends_with(".tar.gz") || name.ends_with(".zip"))
}

pub(crate) fn v2_4_private_identity() -> (&'static str, &'static str) {
    (V2_4_PRIVATE_NETWORK_PROFILE, V2_4_PRIVATE_CHAIN_ID)
}

pub(crate) fn is_approved_v2_4_provenance(proof: &TrustedBinaryProvenance) -> bool {
    let Some(final_release_commit) = V2_4_FINAL_RELEASE_COMMIT else {
        return false;
    };
    proof.release_tag == V2_4_CANDIDATE_TAG && proof.source_commit == final_release_commit
}

pub(crate) fn v2_4_private_state_marker_contents() -> String {
    format!(
        "network_profile={V2_4_PRIVATE_NETWORK_PROFILE}\nchain_id={V2_4_PRIVATE_CHAIN_ID}\nrelease={V2_4_CANDIDATE_TAG}\n"
    )
}

fn supported_v2_4_candidate_target() -> Result<&'static str, String> {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        return Ok("x86_64-unknown-linux-gnu");
    }
    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    {
        return Ok("x86_64-pc-windows-msvc");
    }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        return Ok("x86_64-apple-darwin");
    }
    #[allow(unreachable_code)]
    Err("PulseDAG v2.4.0 does not define a candidate archive for this desktop target.".into())
}

fn v2_4_candidate_archive_layout(
    file_name: &str,
    binary_kind: V2_4CandidateBinaryKind,
) -> Result<ProvenanceArchiveLayout, String> {
    let (base_name, kind) = if let Some(base) = file_name.strip_suffix(".tar.gz") {
        (base.to_string(), ProvenanceArchiveKind::TarGz)
    } else if let Some(base) = file_name.strip_suffix(".zip") {
        (base.to_string(), ProvenanceArchiveKind::Zip)
    } else {
        return Err("A v2.4 candidate archive must be a .tar.gz or .zip file.".into());
    };

    let prefix = format!("{}-{V2_4_CANDIDATE_TAG}-", binary_kind.archive_prefix());
    let target = base_name
        .strip_prefix(&prefix)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "The archive name does not identify the PulseDAG v2.4.0 {} candidate.",
                binary_kind.public_name()
            )
        })?
        .to_string();

    if !matches!(
        target.as_str(),
        "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "x86_64-apple-darwin"
    ) {
        return Err("The v2.4 candidate archive target is not in the release workflow allowlist.".into());
    }

    let windows_target = target == "x86_64-pc-windows-msvc";
    match (kind, windows_target) {
        (ProvenanceArchiveKind::Zip, true) | (ProvenanceArchiveKind::TarGz, false) => {}
        _ => return Err("The v2.4 candidate archive format does not match its target.".into()),
    }

    let binary_name = match (binary_kind, windows_target) {
        (V2_4CandidateBinaryKind::Node, true) => "pulsedagd.exe",
        (V2_4CandidateBinaryKind::Node, false) => "pulsedagd",
        (V2_4CandidateBinaryKind::Miner, true) => "pulsedag-miner.exe",
        (V2_4CandidateBinaryKind::Miner, false) => "pulsedag-miner",
    }
    .to_string();
    let root = PathBuf::from(&base_name);
    let binary_path = root.join(&binary_name);
    let allowed_files = [
        binary_path.clone(),
        root.join("README.md"),
        root.join(V2_4_INSTALL_GUIDE),
    ]
    .into_iter()
    .collect();

    Ok(ProvenanceArchiveLayout {
        base_name,
        target,
        binary_name,
        binary_path,
        allowed_files,
        kind,
    })
}

fn inspect_v2_4_candidate_archive_path(
    path: &Path,
    binary_kind: V2_4CandidateBinaryKind,
) -> Result<V2_4CandidateArchiveInspection, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve the v2.4 candidate archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect the v2.4 candidate archive: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_PROVENANCE_ARCHIVE_BYTES {
        return Err("The v2.4 candidate archive is not a regular file within the safety limit.".into());
    }

    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The v2.4 candidate archive name is not valid UTF-8.".to_string())?;
    let layout = v2_4_candidate_archive_layout(file_name, binary_kind)?;
    let host_target = supported_v2_4_candidate_target()?;
    if layout.target != host_target {
        return Err(format!(
            "The v2.4 candidate target {} does not match this desktop target {host_target}.",
            layout.target
        ));
    }

    let mut file = File::open(&canonical)
        .map_err(|error| format!("Cannot open the v2.4 candidate archive: {error}"))?;
    let (archive_sha256, archive_size_bytes) = hash_reader_limited(
        &mut file,
        MAX_PROVENANCE_ARCHIVE_BYTES,
        "the v2.4 candidate archive",
    )?;
    if archive_size_bytes != metadata.len() {
        return Err("The v2.4 candidate archive changed while it was being inspected.".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Cannot rewind the v2.4 candidate archive: {error}"))?;

    let evidence = match layout.kind {
        ProvenanceArchiveKind::Zip => inspect_zip_binary(file, &layout, archive_sha256.clone()),
        ProvenanceArchiveKind::TarGz => inspect_tar_binary(file, &layout, archive_sha256.clone()),
    }?;

    Ok(V2_4CandidateArchiveInspection {
        archive_name: file_name.to_string(),
        archive_sha256,
        archive_size_bytes,
        binary_kind: binary_kind.public_name().to_string(),
        release_tag: V2_4_CANDIDATE_TAG.to_string(),
        source_commit: V2_4_FINAL_RELEASE_COMMIT.unwrap_or("unfrozen").to_string(),
        target: evidence.target,
        embedded_path: evidence.embedded_path,
        embedded_binary_sha256: evidence.binary_sha256,
        embedded_binary_size_bytes: evidence.binary_size_bytes,
        structurally_valid: true,
        approved: false,
        message: "The archive matches the local PulseDAG v2.4.0 candidate layout and safety bounds, but Task31 has not frozen a final release SHA. This evidence is structural only and cannot become trusted provenance or authorize private launch."
            .into(),
    })
}

#[tauri::command]
fn inspect_v2_4_candidate_archive(
    path: String,
    binary_kind: String,
) -> Result<V2_4CandidateArchiveInspection, String> {
    let kind = V2_4CandidateBinaryKind::parse(&binary_kind)?;
    inspect_v2_4_candidate_archive_path(Path::new(path.trim()), kind)
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
    fn v2_4_candidate_identity_is_unfrozen_and_fail_closed() {
        assert_eq!(V2_4_CANDIDATE_TAG, "v2.4.0");
        assert_eq!(
            V2_4_STALE_RELEASE_PREP_COMMIT,
            "995b7b200afc90f705eece6c37a16b7a4fc294ec"
        );
        assert!(V2_4_FINAL_RELEASE_COMMIT.is_none());
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
    fn v2_4_candidate_layout_is_exact_for_node_and_miner() {
        let node = v2_4_candidate_archive_layout(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip",
            V2_4CandidateBinaryKind::Node,
        )
        .unwrap();
        assert_eq!(node.target, "x86_64-pc-windows-msvc");
        assert_eq!(node.binary_name, "pulsedagd.exe");
        assert!(node.allowed_files.contains(&PathBuf::from(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc/INSTALL_BINARIES_V2_4_0.md"
        )));
        assert_eq!(node.allowed_files.len(), 3);

        let miner = v2_4_candidate_archive_layout(
            "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
            V2_4CandidateBinaryKind::Miner,
        )
        .unwrap();
        assert_eq!(miner.target, "x86_64-unknown-linux-gnu");
        assert_eq!(miner.binary_name, "pulsedag-miner");
        assert_eq!(miner.allowed_files.len(), 3);
    }

    #[test]
    fn v2_4_candidate_layout_rejects_wrong_kind_target_and_format() {
        assert!(v2_4_candidate_archive_layout(
            "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
            V2_4CandidateBinaryKind::Node,
        )
        .is_err());
        assert!(v2_4_candidate_archive_layout(
            "pulsedagd-v2.4.0-aarch64-unknown-linux-gnu.tar.gz",
            V2_4CandidateBinaryKind::Node,
        )
        .is_err());
        assert!(v2_4_candidate_archive_layout(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.tar.gz",
            V2_4CandidateBinaryKind::Node,
        )
        .is_err());
        assert!(v2_4_candidate_archive_layout(
            "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.zip",
            V2_4CandidateBinaryKind::Miner,
        )
        .is_err());
    }

    #[test]
    fn v2_4_candidate_binary_kind_is_closed() {
        assert_eq!(
            V2_4CandidateBinaryKind::parse("node").unwrap(),
            V2_4CandidateBinaryKind::Node
        );
        assert_eq!(
            V2_4CandidateBinaryKind::parse("miner").unwrap(),
            V2_4CandidateBinaryKind::Miner
        );
        assert!(V2_4CandidateBinaryKind::parse("wallet").is_err());
        assert!(V2_4CandidateBinaryKind::parse("anything").is_err());
    }

    #[test]
    fn v2_4_private_identity_rejects_all_provenance_until_task31_freeze() {
        assert!(!is_approved_v2_4_provenance(&proof(
            V2_4_CANDIDATE_TAG,
            V2_4_STALE_RELEASE_PREP_COMMIT
        )));
        assert!(!is_approved_v2_4_provenance(&proof(
            "v2.3.0",
            "7e43225f01ac05d15e5f1e3f1550d7850bf18cbc"
        )));
        assert!(!is_approved_v2_4_provenance(&proof(
            V2_4_CANDIDATE_TAG,
            "91dd8f4314cd0a0672cf3c98f00eea039e59e429"
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
