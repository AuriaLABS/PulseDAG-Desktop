pub(crate) const V2_4_PRIVATE_PROTOCOL_CONSENSUS_MODE: &str = "ghostdag_v1";
pub(crate) const V2_4_PRIVATE_RUNTIME_CONSENSUS_MODE: &str = "legacy";
const V2_4_PRIVATE_STATE_MARKER_MAX_BYTES: u64 = 4 * 1024;

pub(crate) fn v2_4_private_environment() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PULSEDAG_SINGLE_NODE_MODE", "true"),
        ("PULSEDAG_PRIVATE_TESTNET_ROLE", "single"),
        ("PULSEDAG_NETWORK_PROFILE", V2_4_PRIVATE_NETWORK_PROFILE),
        ("PULSEDAG_CHAIN_ID", V2_4_PRIVATE_CHAIN_ID),
        ("PULSEDAG_CONSENSUS_MODE", V2_4_PRIVATE_RUNTIME_CONSENSUS_MODE),
        (
            "PULSEDAG_PROTOCOL_CONSENSUS_MODE",
            V2_4_PRIVATE_PROTOCOL_CONSENSUS_MODE,
        ),
        ("PULSEDAG_P2P_ENABLED", "false"),
        ("PULSEDAG_P2P_MODE", "libp2p-real"),
        ("PULSEDAG_P2P_BOOTSTRAP", ""),
        ("PULSEDAG_PUBLIC_P2P_MULTIADDR", ""),
        ("PULSEDAG_P2P_MDNS", "false"),
        ("PULSEDAG_P2P_KADEMLIA", "true"),
        ("PULSEDAG_AUTO_PRUNE_ENABLED", "false"),
        ("PULSEDAG_PRUNE_REQUIRE_SNAPSHOT", "true"),
        ("PULSEDAG_PUBLIC_TESTNET_READY", "false"),
        (
            "PULSEDAG_THIRTY_DAY_PUBLIC_TESTNET_CLOCK_STARTED",
            "false",
        ),
        ("PULSEDAG_MULTI_HOST_REHEARSAL", "false"),
        ("PULSEDAG_CONTRACTS_ENABLED", "false"),
    ]
}

pub(crate) fn apply_v2_4_private_environment(command: &mut Command) {
    for (key, value) in v2_4_private_environment() {
        command.env(key, value);
    }
}

pub(crate) fn ensure_v2_4_private_state_boundary(data_directory: &Path) -> Result<(), String> {
    let marker_path = data_directory.join(V2_4_PRIVATE_STATE_MARKER);
    let expected = v2_4_private_state_marker_contents();

    if marker_path.exists() {
        let metadata = fs::metadata(&marker_path)
            .map_err(|error| format!("Cannot inspect the v2.4 private-state marker: {error}"))?;
        if !metadata.is_file() || metadata.len() > V2_4_PRIVATE_STATE_MARKER_MAX_BYTES {
            return Err(
                "The v2.4 private-state marker is not a bounded regular file. Refusing to use this data directory."
                    .into(),
            );
        }
        let contents = fs::read_to_string(&marker_path)
            .map_err(|error| format!("Cannot read the v2.4 private-state marker: {error}"))?;
        if contents != expected {
            return Err(
                "The data directory is marked for a different PulseDAG private identity or release. Refusing to reuse it as v2.4 state."
                    .into(),
            );
        }
        return Ok(());
    }

    let mut entries = fs::read_dir(data_directory)
        .map_err(|error| format!("Cannot inspect the private data directory: {error}"))?;
    if entries.next().transpose().map_err(|error| {
        format!("Cannot inspect an entry in the private data directory: {error}")
    })?.is_some()
    {
        return Err(
            "The selected private data directory already contains unmarked state. Choose a new empty directory for PulseDAG v2.4.0; Desktop will not relabel or delete earlier v2.3/private state."
                .into(),
        );
    }

    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker_path)
        .map_err(|error| format!("Cannot create the v2.4 private-state marker: {error}"))?;
    marker
        .write_all(expected.as_bytes())
        .map_err(|error| format!("Cannot write the v2.4 private-state marker: {error}"))?;
    marker
        .sync_all()
        .map_err(|error| format!("Cannot persist the v2.4 private-state marker: {error}"))?;
    Ok(())
}

fn v2_4_node_archive_kind_is_exact(inspected: &V2_4FinalArchiveInspection) -> bool {
    inspected.binary_kind == V2_4CandidateBinaryKind::Node
        && inspected.archive_name.starts_with("pulsedagd-v2.4.0-")
}

fn v2_4_miner_archive_kind_is_exact(inspected: &V2_4FinalArchiveInspection) -> bool {
    inspected.binary_kind == V2_4CandidateBinaryKind::Miner
        && inspected.archive_name.starts_with("pulsedag-miner-v2.4.0-")
}

pub(crate) fn is_final_v2_4_node_provenance(proof: &TrustedBinaryProvenance) -> bool {
    proof.archive_name.starts_with("pulsedagd-v2.4.0-")
        && is_final_v2_4_release_provenance(proof)
}

pub(crate) fn is_final_v2_4_miner_provenance(proof: &TrustedBinaryProvenance) -> bool {
    proof.archive_name.starts_with("pulsedag-miner-v2.4.0-")
        && is_final_v2_4_release_provenance(proof)
}

fn public_v2_4_provenance(
    proof: &TrustedBinaryProvenance,
    binary_kind: V2_4CandidateBinaryKind,
) -> BinaryProvenance {
    BinaryProvenance {
        archive_name: proof.archive_name.clone(),
        archive_sha256: proof.archive_sha256.clone(),
        release_tag: proof.release_tag.clone(),
        source_commit: proof.source_commit.clone(),
        target: proof.target.clone(),
        embedded_path: proof.embedded_path.clone(),
        embedded_binary_sha256: proof.binary_sha256.clone(),
        embedded_binary_size_bytes: proof.binary_size_bytes,
        selected_binary_sha256: proof.binary_sha256.clone(),
        selected_binary_size_bytes: proof.binary_size_bytes,
        linked_at_ms: proof.linked_at_ms,
        approved: true,
        message: format!(
            "The selected {} is byte-for-byte identical to the final PulseDAG v2.4.0 release binary and is bound to Task31 provenance.",
            binary_kind.public_name()
        ),
    }
}

#[tauri::command]
async fn bind_v2_4_node_binary_to_verified_archive(
    archive_path: String,
    executable_path: String,
    state: State<'_, NodeSupervisor>,
) -> Result<BinaryProvenance, String> {
    let release = verify_v2_4_release_archive(archive_path.clone(), "node".into()).await?;
    if !release.approved {
        return Err(release.message);
    }

    let task_archive = archive_path.clone();
    let task_binary = executable_path.clone();
    let (inspected, selected) = tauri::async_runtime::spawn_blocking(move || {
        let inspected = inspect_v2_4_final_archive_path(
            Path::new(task_archive.trim()),
            V2_4CandidateBinaryKind::Node,
        )?;
        let selected = validate_binary_path(Path::new(task_binary.trim()))?;
        Ok::<_, String>((inspected, selected))
    })
    .await
    .map_err(|error| format!("v2.4 node provenance task failed: {error}"))??;

    if !v2_4_node_archive_kind_is_exact(&inspected)
        || inspected.archive_name != release.archive_name
        || inspected.archive_size_bytes != release.size_bytes
        || !inspected.archive_sha256.eq_ignore_ascii_case(&release.sha256)
    {
        return Err("The v2.4 node archive changed after final release verification.".into());
    }
    if !inspected
        .embedded_binary_sha256
        .eq_ignore_ascii_case(&selected.sha256)
        || inspected.embedded_binary_size_bytes != selected.size_bytes
    {
        state
            .provenance
            .lock()
            .map_err(|_| "Binary provenance state is unavailable.".to_string())?
            .take();
        return Err(
            "The selected pulsedagd executable is not byte-for-byte identical to the final v2.4.0 archive member."
                .into(),
        );
    }

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
    if !is_final_v2_4_node_provenance(&proof) {
        return Err("The node proof does not satisfy the final v2.4.0 Task31 provenance gate.".into());
    }
    let result = public_v2_4_provenance(&proof, V2_4CandidateBinaryKind::Node);
    *state
        .provenance
        .lock()
        .map_err(|_| "Binary provenance state is unavailable.".to_string())? = Some(proof);
    Ok(result)
}

#[tauri::command]
async fn bind_v2_4_miner_binary_to_verified_archive(
    archive_path: String,
    executable_path: String,
    state: State<'_, MinerProvenanceRegistry>,
) -> Result<BinaryProvenance, String> {
    let release = verify_v2_4_release_archive(archive_path.clone(), "miner".into()).await?;
    if !release.approved {
        return Err(release.message);
    }

    let task_archive = archive_path.clone();
    let task_binary = executable_path.clone();
    let (inspected, selected) = tauri::async_runtime::spawn_blocking(move || {
        let inspected = inspect_v2_4_final_archive_path(
            Path::new(task_archive.trim()),
            V2_4CandidateBinaryKind::Miner,
        )?;
        let selected = validate_miner_binary_path(Path::new(task_binary.trim()))?;
        Ok::<_, String>((inspected, selected))
    })
    .await
    .map_err(|error| format!("v2.4 miner provenance task failed: {error}"))??;

    if !v2_4_miner_archive_kind_is_exact(&inspected)
        || inspected.archive_name != release.archive_name
        || inspected.archive_size_bytes != release.size_bytes
        || !inspected.archive_sha256.eq_ignore_ascii_case(&release.sha256)
    {
        return Err("The v2.4 miner archive changed after final release verification.".into());
    }
    if !inspected
        .embedded_binary_sha256
        .eq_ignore_ascii_case(&selected.sha256)
        || inspected.embedded_binary_size_bytes != selected.size_bytes
    {
        state
            .provenance
            .lock()
            .map_err(|_| "Miner provenance state is unavailable.".to_string())?
            .take();
        return Err(
            "The selected pulsedag-miner executable is not byte-for-byte identical to the final v2.4.0 archive member."
                .into(),
        );
    }

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
    if !is_final_v2_4_miner_provenance(&proof) {
        return Err("The miner proof does not satisfy the final v2.4.0 Task31 provenance gate.".into());
    }
    let result = public_v2_4_provenance(&proof, V2_4CandidateBinaryKind::Miner);
    *state
        .provenance
        .lock()
        .map_err(|_| "Miner provenance state is unavailable.".to_string())? = Some(proof);
    Ok(result)
}

#[cfg(test)]
mod v2_4_private_tests {
    use super::*;

    fn temp_private_dir(label: &str) -> PathBuf {
        let path = env::temp_dir().join(format!(
            "pulsedag-desktop-v24-private-{}-{}-{label}",
            std::process::id(),
            unix_time_ms()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary private state directory");
        path
    }

    fn final_proof(archive_name: &str, binary_kind: V2_4CandidateBinaryKind) -> TrustedBinaryProvenance {
        let layout = v2_4_final_archive_layout(archive_name, binary_kind).expect("final layout");
        TrustedBinaryProvenance {
            executable_path: "/tmp/release-binary".into(),
            binary_sha256: "a".repeat(64),
            binary_size_bytes: 4,
            archive_name: archive_name.into(),
            archive_sha256: v2_4_final_archive_digest(archive_name)
                .expect("frozen digest")
                .into(),
            release_tag: V2_4_FINAL_RELEASE_TAG.into(),
            source_commit: V2_4_FINAL_RELEASE_SOURCE_COMMIT.into(),
            target: layout.target,
            embedded_path: layout.binary_path.display().to_string(),
            linked_at_ms: 1,
        }
    }

    #[test]
    fn v2_4_private_environment_is_isolated_and_fail_closed() {
        let values = v2_4_private_environment()
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(values.get("PULSEDAG_SINGLE_NODE_MODE"), Some(&"true"));
        assert_eq!(values.get("PULSEDAG_PRIVATE_TESTNET_ROLE"), Some(&"single"));
        assert_eq!(
            values.get("PULSEDAG_NETWORK_PROFILE"),
            Some(&V2_4_PRIVATE_NETWORK_PROFILE)
        );
        assert_eq!(values.get("PULSEDAG_CHAIN_ID"), Some(&V2_4_PRIVATE_CHAIN_ID));
        assert_eq!(values.get("PULSEDAG_CONSENSUS_MODE"), Some(&"legacy"));
        assert_eq!(
            values.get("PULSEDAG_PROTOCOL_CONSENSUS_MODE"),
            Some(&"ghostdag_v1")
        );
        assert_eq!(values.get("PULSEDAG_P2P_ENABLED"), Some(&"false"));
        assert_eq!(values.get("PULSEDAG_PUBLIC_TESTNET_READY"), Some(&"false"));
        assert_eq!(
            values.get("PULSEDAG_THIRTY_DAY_PUBLIC_TESTNET_CLOCK_STARTED"),
            Some(&"false")
        );
        assert_eq!(values.get("PULSEDAG_CONTRACTS_ENABLED"), Some(&"false"));
    }

    #[test]
    fn v2_4_private_state_marker_accepts_only_exact_identity() {
        let directory = temp_private_dir("exact-marker");
        ensure_v2_4_private_state_boundary(&directory).expect("mark empty v2.4 directory");
        ensure_v2_4_private_state_boundary(&directory).expect("reuse exact v2.4 marker");
        assert_eq!(
            fs::read_to_string(directory.join(V2_4_PRIVATE_STATE_MARKER)).unwrap(),
            v2_4_private_state_marker_contents()
        );
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn v2_4_private_state_boundary_rejects_unmarked_existing_state() {
        let directory = temp_private_dir("old-state");
        fs::create_dir(directory.join("rocksdb")).expect("create simulated old state");
        let error = ensure_v2_4_private_state_boundary(&directory)
            .expect_err("unmarked pre-existing state must fail closed");
        assert!(error.contains("already contains unmarked state"));
        assert!(!directory.join(V2_4_PRIVATE_STATE_MARKER).exists());
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn v2_4_private_state_boundary_rejects_mismatched_marker() {
        let directory = temp_private_dir("wrong-marker");
        fs::write(
            directory.join(V2_4_PRIVATE_STATE_MARKER),
            "network_profile=private\nchain_id=pulsedag-private\n",
        )
        .expect("write stale marker");
        assert!(ensure_v2_4_private_state_boundary(&directory).is_err());
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn v2_4_private_provenance_is_binary_family_bound() {
        let node = final_proof(
            "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
            V2_4CandidateBinaryKind::Node,
        );
        let miner = final_proof(
            "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
            V2_4CandidateBinaryKind::Miner,
        );
        assert!(is_final_v2_4_node_provenance(&node));
        assert!(!is_final_v2_4_miner_provenance(&node));
        assert!(is_final_v2_4_miner_provenance(&miner));
        assert!(!is_final_v2_4_node_provenance(&miner));
    }
}
