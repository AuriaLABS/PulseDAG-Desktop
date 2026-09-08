pub(crate) const V2_4_FINAL_PRIVATE_STATE_MARKER: &str =
    ".pulsedag-desktop-v2.4-final";
const V2_4_FINAL_PRIVATE_STATE_MARKER_MAX_BYTES: u64 = 4 * 1024;

pub(crate) fn v2_4_final_private_state_marker_contents() -> String {
    format!(
        concat!(
            "network_profile={}\n",
            "chain_id={}\n",
            "release={}\n",
            "source_commit={}\n",
            "source_tree={}\n",
            "runtime_consensus={}\n",
            "protocol_consensus={}\n",
            "single_node=true\n",
            "p2p=false\n",
            "public_testnet=false\n",
            "contracts=false\n"
        ),
        V2_4_PRIVATE_NETWORK_PROFILE,
        V2_4_PRIVATE_CHAIN_ID,
        V2_4_FINAL_RELEASE_TAG,
        V2_4_FINAL_RELEASE_SOURCE_COMMIT,
        V2_4_FINAL_RELEASE_SOURCE_TREE,
        V2_4_PRIVATE_RUNTIME_CONSENSUS_MODE,
        V2_4_PRIVATE_PROTOCOL_CONSENSUS_MODE,
    )
}

pub(crate) fn ensure_v2_4_final_private_state_binding(
    data_directory: &Path,
) -> Result<(), String> {
    let marker_path = data_directory.join(V2_4_FINAL_PRIVATE_STATE_MARKER);
    let expected = v2_4_final_private_state_marker_contents();

    if marker_path.exists() {
        let metadata = fs::metadata(&marker_path)
            .map_err(|error| format!("Cannot inspect the final v2.4 state marker: {error}"))?;
        if !metadata.is_file() || metadata.len() > V2_4_FINAL_PRIVATE_STATE_MARKER_MAX_BYTES {
            return Err(
                "The final v2.4 private-state marker is not a bounded regular file. Refusing to use this data directory."
                    .into(),
            );
        }
        let contents = fs::read_to_string(&marker_path)
            .map_err(|error| format!("Cannot read the final v2.4 state marker: {error}"))?;
        if contents != expected {
            return Err(
                "The data directory is bound to a different PulseDAG source, protocol, or private safety contract. Refusing to reuse it."
                    .into(),
            );
        }
        return Ok(());
    }

    let mut unexpected = Vec::new();
    for entry in fs::read_dir(data_directory)
        .map_err(|error| format!("Cannot inspect the private data directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Cannot inspect an entry in the private data directory: {error}"))?;
        let name = entry.file_name();
        if name != std::ffi::OsStr::new(V2_4_PRIVATE_STATE_MARKER) {
            unexpected.push(name.to_string_lossy().into_owned());
        }
    }
    if !unexpected.is_empty() {
        return Err(
            "The selected directory has v2.4 identity metadata but no final source/protocol binding and already contains state. Choose a new empty directory; Desktop will not retroactively approve or migrate it."
                .into(),
        );
    }

    let identity_marker = data_directory.join(V2_4_PRIVATE_STATE_MARKER);
    if !identity_marker.is_file() {
        return Err(
            "The v2.4 identity marker must exist before final private-state binding is created."
                .into(),
        );
    }

    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker_path)
        .map_err(|error| format!("Cannot create the final v2.4 state marker: {error}"))?;
    marker
        .write_all(expected.as_bytes())
        .map_err(|error| format!("Cannot write the final v2.4 state marker: {error}"))?;
    marker
        .sync_all()
        .map_err(|error| format!("Cannot persist the final v2.4 state marker: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod v2_4_final_state_binding_tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let path = env::temp_dir().join(format!(
            "pulsedag-desktop-v24-final-state-{}-{}-{label}",
            std::process::id(),
            unix_time_ms()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary final-state directory");
        path
    }

    #[test]
    fn final_state_marker_binds_source_tree_and_protocol() {
        let contents = v2_4_final_private_state_marker_contents();
        assert!(contents.contains(V2_4_FINAL_RELEASE_SOURCE_COMMIT));
        assert!(contents.contains(V2_4_FINAL_RELEASE_SOURCE_TREE));
        assert!(contents.contains("runtime_consensus=legacy"));
        assert!(contents.contains("protocol_consensus=ghostdag_v1"));
        assert!(contents.contains("public_testnet=false"));
        assert!(contents.contains("contracts=false"));
    }

    #[test]
    fn final_state_binding_is_created_only_before_runtime_state_exists() {
        let directory = temp_dir("fresh");
        ensure_v2_4_private_state_boundary(&directory).expect("create identity marker");
        ensure_v2_4_final_private_state_binding(&directory).expect("bind final state");
        ensure_v2_4_final_private_state_binding(&directory).expect("reuse exact final binding");
        assert_eq!(
            fs::read_to_string(directory.join(V2_4_FINAL_PRIVATE_STATE_MARKER)).unwrap(),
            v2_4_final_private_state_marker_contents()
        );
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn final_state_binding_refuses_to_bless_preexisting_state() {
        let directory = temp_dir("preexisting");
        fs::write(
            directory.join(V2_4_PRIVATE_STATE_MARKER),
            v2_4_private_state_marker_contents(),
        )
        .expect("write identity marker");
        fs::create_dir(directory.join("rocksdb")).expect("create simulated state");
        let error = ensure_v2_4_final_private_state_binding(&directory)
            .expect_err("preexisting state without final binding must fail closed");
        assert!(error.contains("will not retroactively approve or migrate"));
        assert!(!directory.join(V2_4_FINAL_PRIVATE_STATE_MARKER).exists());
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn final_state_binding_rejects_changed_source_or_protocol() {
        let directory = temp_dir("mismatch");
        fs::write(
            directory.join(V2_4_PRIVATE_STATE_MARKER),
            v2_4_private_state_marker_contents(),
        )
        .expect("write identity marker");
        fs::write(
            directory.join(V2_4_FINAL_PRIVATE_STATE_MARKER),
            "source_commit=old\nprotocol_consensus=legacy\n",
        )
        .expect("write stale final marker");
        assert!(ensure_v2_4_final_private_state_binding(&directory).is_err());
        fs::remove_dir_all(directory).ok();
    }
}
