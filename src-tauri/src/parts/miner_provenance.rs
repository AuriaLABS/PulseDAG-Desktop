#[derive(Default)]
struct MinerProvenanceRegistry {
    provenance: Mutex<Option<TrustedBinaryProvenance>>,
}

fn miner_provenance_archive_layout(file_name: &str) -> Result<ProvenanceArchiveLayout, String> {
    let (base_name, kind) = if let Some(base) = file_name.strip_suffix(".tar.gz") {
        (base.to_string(), ProvenanceArchiveKind::TarGz)
    } else if let Some(base) = file_name.strip_suffix(".zip") {
        (base.to_string(), ProvenanceArchiveKind::Zip)
    } else {
        return Err("The approved miner release archive must be a .tar.gz or .zip file.".into());
    };
    let target = base_name
        .strip_prefix("pulsedag-miner-v2.3.0-")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "The release archive name does not identify pulsedag-miner v2.3.0.".to_string()
        })?
        .to_string();
    if !matches!(
        target.as_str(),
        "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "x86_64-apple-darwin"
    ) {
        return Err("The miner release archive target is not approved for PulseDAG v2.3.0.".into());
    }
    let windows_target = target == "x86_64-pc-windows-msvc";
    match (kind, windows_target) {
        (ProvenanceArchiveKind::Zip, true) | (ProvenanceArchiveKind::TarGz, false) => {}
        _ => return Err("The miner release archive format does not match its declared target.".into()),
    }
    let binary_name = if windows_target {
        "pulsedag-miner.exe"
    } else {
        "pulsedag-miner"
    }
    .to_string();
    let root = PathBuf::from(&base_name);
    let binary_path = root.join(&binary_name);
    let allowed_files = [
        binary_path.clone(),
        root.join("README.md"),
        root.join("INSTALL_BINARIES_V2_3_0.md"),
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

fn inspect_miner_release_archive_metadata(path: &Path) -> Result<LocalReleaseArchive, String> {
    if path.as_os_str().is_empty() {
        return Err("Select a PulseDAG miner release archive first.".into());
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve miner release archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect miner release archive: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_PROVENANCE_ARCHIVE_BYTES {
        return Err("The selected miner archive is not a regular file within the safety limit.".into());
    }
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The miner release archive name is not valid UTF-8.".to_string())?
        .to_string();
    miner_provenance_archive_layout(&file_name)?;
    Ok(LocalReleaseArchive {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        sha256: sha256_path(&canonical)?,
    })
}

fn validate_miner_archive_file_set(
    layout: &ProvenanceArchiveLayout,
    seen_files: &HashSet<PathBuf>,
) -> Result<(), String> {
    if seen_files != &layout.allowed_files {
        let missing = layout
            .allowed_files
            .difference(seen_files)
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        return Err(format!(
            "The miner release archive has an unexpected file set{}.",
            if missing.is_empty() {
                String::new()
            } else {
                format!("; missing {}", missing.join(", "))
            }
        ));
    }
    Ok(())
}

fn inspect_miner_zip_binary(
    file: File,
    layout: &ProvenanceArchiveLayout,
    archive_sha256: String,
) -> Result<EmbeddedBinaryEvidence, String> {
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("Cannot open the approved miner ZIP archive: {error}"))?;
    if archive.len() > MAX_PROVENANCE_ENTRIES {
        return Err("The miner release ZIP contains too many entries.".into());
    }
    let mut seen_files = HashSet::new();
    let mut binary_evidence = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Cannot inspect miner ZIP entry {index}: {error}"))?;
        let path = entry
            .enclosed_name()
            .ok_or_else(|| "The miner release ZIP contains an unsafe path.".to_string())?
            .to_path_buf();
        if !safe_provenance_entry_path(&path) {
            return Err("The miner release ZIP contains an unsafe path.".into());
        }
        if entry.is_dir() {
            if path != PathBuf::from(&layout.base_name) {
                return Err("The miner release ZIP contains an unexpected directory.".into());
            }
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("The miner release ZIP contains a symbolic link.".into());
        }
        if !layout.allowed_files.contains(&path) || !seen_files.insert(path.clone()) {
            return Err("The miner release ZIP contains an unexpected or duplicate file.".into());
        }
        let declared_size = entry.size();
        if path == layout.binary_path {
            if declared_size > MAX_PROVENANCE_BINARY_BYTES {
                return Err("The embedded pulsedag-miner binary exceeds the safety limit.".into());
            }
            let (binary_sha256, binary_size_bytes) = hash_reader_limited(
                &mut entry,
                MAX_PROVENANCE_BINARY_BYTES,
                "the embedded pulsedag-miner binary",
            )?;
            if binary_size_bytes != declared_size {
                return Err("The embedded pulsedag-miner size does not match the ZIP metadata.".into());
            }
            binary_evidence = Some(EmbeddedBinaryEvidence {
                target: layout.target.clone(),
                embedded_path: path.display().to_string(),
                archive_sha256: archive_sha256.clone(),
                binary_sha256,
                binary_size_bytes,
            });
        } else if declared_size > MAX_PROVENANCE_DOCUMENT_BYTES {
            return Err("A miner release document exceeds the safety limit.".into());
        }
    }
    validate_miner_archive_file_set(layout, &seen_files)?;
    binary_evidence.ok_or_else(|| "The approved ZIP does not contain pulsedag-miner.".into())
}

fn inspect_miner_tar_binary(
    file: File,
    layout: &ProvenanceArchiveLayout,
    archive_sha256: String,
) -> Result<EmbeddedBinaryEvidence, String> {
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut seen_files = HashSet::new();
    let mut binary_evidence = None;
    let mut entry_count = 0_usize;
    let entries = archive
        .entries()
        .map_err(|error| format!("Cannot open the approved miner TAR.GZ archive: {error}"))?;
    for entry in entries {
        entry_count += 1;
        if entry_count > MAX_PROVENANCE_ENTRIES {
            return Err("The miner release TAR.GZ contains too many entries.".into());
        }
        let mut entry = entry.map_err(|error| format!("Cannot inspect miner TAR entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("Cannot decode a miner TAR entry path: {error}"))?
            .into_owned();
        if !safe_provenance_entry_path(&path) {
            return Err("The miner release TAR.GZ contains an unsafe path.".into());
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            if path != PathBuf::from(&layout.base_name) {
                return Err("The miner release TAR.GZ contains an unexpected directory.".into());
            }
            continue;
        }
        if !entry_type.is_file() {
            return Err("The miner release TAR.GZ contains a link or unsupported entry type.".into());
        }
        if !layout.allowed_files.contains(&path) || !seen_files.insert(path.clone()) {
            return Err("The miner release TAR.GZ contains an unexpected or duplicate file.".into());
        }
        let declared_size = entry.size();
        if path == layout.binary_path {
            if declared_size > MAX_PROVENANCE_BINARY_BYTES {
                return Err("The embedded pulsedag-miner binary exceeds the safety limit.".into());
            }
            let (binary_sha256, binary_size_bytes) = hash_reader_limited(
                &mut entry,
                MAX_PROVENANCE_BINARY_BYTES,
                "the embedded pulsedag-miner binary",
            )?;
            if binary_size_bytes != declared_size {
                return Err("The embedded pulsedag-miner size does not match the TAR metadata.".into());
            }
            binary_evidence = Some(EmbeddedBinaryEvidence {
                target: layout.target.clone(),
                embedded_path: path.display().to_string(),
                archive_sha256: archive_sha256.clone(),
                binary_sha256,
                binary_size_bytes,
            });
        } else if declared_size > MAX_PROVENANCE_DOCUMENT_BYTES {
            return Err("A miner release document exceeds the safety limit.".into());
        }
    }
    validate_miner_archive_file_set(layout, &seen_files)?;
    binary_evidence.ok_or_else(|| "The approved TAR.GZ does not contain pulsedag-miner.".into())
}

fn inspect_embedded_miner_binary(path: &Path) -> Result<EmbeddedBinaryEvidence, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve the miner release archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect the miner release archive: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_PROVENANCE_ARCHIVE_BYTES {
        return Err("The miner release archive is not a regular file within the safety limit.".into());
    }
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The miner release archive name is not valid UTF-8.".to_string())?;
    let layout = miner_provenance_archive_layout(file_name)?;
    let host_target = supported_host_release_target()?;
    if layout.target != host_target {
        return Err(format!(
            "The miner archive target {} does not match this desktop target {host_target}.",
            layout.target
        ));
    }
    let mut file = File::open(&canonical)
        .map_err(|error| format!("Cannot open the miner release archive: {error}"))?;
    let (archive_sha256, archive_size_bytes) = hash_reader_limited(
        &mut file,
        MAX_PROVENANCE_ARCHIVE_BYTES,
        "the miner release archive",
    )?;
    if archive_size_bytes != metadata.len() {
        return Err("The miner release archive changed while it was being inspected.".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Cannot rewind the miner release archive: {error}"))?;
    match layout.kind {
        ProvenanceArchiveKind::Zip => inspect_miner_zip_binary(file, &layout, archive_sha256),
        ProvenanceArchiveKind::TarGz => inspect_miner_tar_binary(file, &layout, archive_sha256),
    }
}

fn public_miner_binary_provenance(proof: &TrustedBinaryProvenance) -> BinaryProvenance {
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
        message: "The selected executable matches pulsedag-miner inside the approved release archive."
            .into(),
    }
}

fn verify_miner_provenance_for_launch(
    state: &MinerProvenanceRegistry,
    binary: &BinaryInfo,
    profile: &str,
) -> Result<Option<TrustedBinaryProvenance>, String> {
    let mut guard = state
        .provenance
        .lock()
        .map_err(|_| "Miner provenance state is unavailable.".to_string())?;
    if let Some(proof) = guard.as_ref() {
        if proof.executable_path == binary.path
            && proof.binary_sha256.eq_ignore_ascii_case(&binary.sha256)
            && proof.binary_size_bytes == binary.size_bytes
        {
            return Ok(Some(proof.clone()));
        }
        *guard = None;
    }
    if profile == "private" {
        return Err(
            "Private-profile mining requires pulsedag-miner to be linked to an approved release archive in this desktop session."
                .into(),
        );
    }
    Ok(None)
}

#[tauri::command]
async fn verify_approved_miner_release_archive(
    path: String,
) -> Result<ReleaseVerification, String> {
    let archive_path = path.trim().to_string();
    let archive = tauri::async_runtime::spawn_blocking(move || {
        inspect_miner_release_archive_metadata(Path::new(&archive_path))
    })
    .await
    .map_err(|error| format!("Miner release archive verification task failed: {error}"))??;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("PulseDAG-Desktop/0.1.0")
        .build()
        .map_err(|error| format!("Cannot initialize miner release verification client: {error}"))?;
    let release = client
        .get(APPROVED_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| format!("Cannot query the approved PulseDAG release: {error}"))?
        .error_for_status()
        .map_err(|error| format!("GitHub rejected the release query: {error}"))?
        .json::<GitHubRelease>()
        .await
        .map_err(|error| format!("Cannot decode the approved release metadata: {error}"))?;

    if release.tag_name != APPROVED_RELEASE_TAG {
        return Err("GitHub returned an unexpected PulseDAG release tag.".into());
    }
    if release.target_commitish != APPROVED_RELEASE_COMMIT {
        return Err("The published v2.3.0 release no longer points to the approved source commit.".into());
    }
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == archive.file_name)
        .ok_or_else(|| {
            "The selected miner archive is not an asset of the approved v2.3.0 release."
                .to_string()
        })?;

    let expected = if let Some(digest) = asset.digest.as_deref() {
        digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
            .map(str::to_string)
            .ok_or_else(|| "GitHub returned an unsupported miner asset digest.".to_string())?
    } else {
        let checksum_name = format!("{}.sha256", archive.file_name);
        let checksum_asset = release
            .assets
            .iter()
            .find(|candidate| candidate.name == checksum_name)
            .ok_or_else(|| "The approved release does not include the miner archive checksum asset.".to_string())?;
        let approved_prefix = format!(
            "https://github.com/AuriaLABS/PulseDAG/releases/download/{APPROVED_RELEASE_TAG}/"
        );
        if !checksum_asset.browser_download_url.starts_with(&approved_prefix) {
            return Err("GitHub returned an unexpected miner checksum download URL.".into());
        }
        let checksum = client
            .get(&checksum_asset.browser_download_url)
            .send()
            .await
            .map_err(|error| format!("Cannot download the approved miner checksum: {error}"))?
            .error_for_status()
            .map_err(|error| format!("GitHub rejected the miner checksum download: {error}"))?
            .text()
            .await
            .map_err(|error| format!("Cannot read the approved miner checksum: {error}"))?;
        let mut fields = checksum.split_whitespace();
        let digest = fields
            .next()
            .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
            .ok_or_else(|| "The approved miner checksum file does not contain a valid SHA-256 digest.".to_string())?;
        let named_file = fields
            .next()
            .map(|value| value.trim_start_matches('*'))
            .ok_or_else(|| "The approved miner checksum file does not name its archive.".to_string())?;
        if named_file != archive.file_name {
            return Err("The approved miner checksum file names a different archive.".into());
        }
        digest.to_string()
    };
    let asset_digest = format!("sha256:{expected}");
    let approved = expected.eq_ignore_ascii_case(&archive.sha256);

    Ok(ReleaseVerification {
        archive_path: archive.path,
        archive_name: archive.file_name,
        size_bytes: archive.size_bytes,
        sha256: archive.sha256,
        release_tag: APPROVED_RELEASE_TAG.to_string(),
        source_commit: APPROVED_RELEASE_COMMIT.to_string(),
        asset_digest,
        approved,
        message: if approved {
            "Miner archive digest matches the asset published in the approved PulseDAG v2.3.0 GitHub release."
                .into()
        } else {
            "Miner archive digest does not match the approved GitHub release asset. Do not extract or run it."
                .into()
        },
    })
}

#[tauri::command]
fn get_miner_binary_provenance(
    state: State<'_, MinerProvenanceRegistry>,
) -> Result<Option<BinaryProvenance>, String> {
    let proof = state
        .provenance
        .lock()
        .map_err(|_| "Miner provenance state is unavailable.".to_string())?
        .as_ref()
        .cloned();
    let Some(proof) = proof else {
        return Ok(None);
    };
    match validate_miner_binary_path(Path::new(&proof.executable_path)) {
        Ok(binary)
            if binary.sha256.eq_ignore_ascii_case(&proof.binary_sha256)
                && binary.size_bytes == proof.binary_size_bytes =>
        {
            Ok(Some(public_miner_binary_provenance(&proof)))
        }
        _ => {
            state
                .provenance
                .lock()
                .map_err(|_| "Miner provenance state is unavailable.".to_string())?
                .take();
            Ok(None)
        }
    }
}

#[tauri::command]
async fn bind_miner_binary_to_verified_archive(
    archive_path: String,
    executable_path: String,
    state: State<'_, MinerProvenanceRegistry>,
) -> Result<BinaryProvenance, String> {
    let release = verify_approved_miner_release_archive(archive_path.clone()).await?;
    if !release.approved {
        return Err(release.message);
    }
    let task_archive = archive_path.clone();
    let task_binary = executable_path.clone();
    let (embedded, selected) = tauri::async_runtime::spawn_blocking(move || {
        let embedded = inspect_embedded_miner_binary(Path::new(task_archive.trim()))?;
        let selected = validate_miner_binary_path(Path::new(task_binary.trim()))?;
        Ok::<_, String>((embedded, selected))
    })
    .await
    .map_err(|error| format!("Miner provenance task failed: {error}"))??;
    if !embedded.archive_sha256.eq_ignore_ascii_case(&release.sha256) {
        return Err("The miner release archive changed after its GitHub digest was verified.".into());
    }
    let approved = embedded
        .binary_sha256
        .eq_ignore_ascii_case(&selected.sha256)
        && embedded.binary_size_bytes == selected.size_bytes;
    let linked_at_ms = unix_time_ms();
    let result = BinaryProvenance {
        archive_name: release.archive_name.clone(),
        archive_sha256: release.sha256.clone(),
        release_tag: release.release_tag.clone(),
        source_commit: release.source_commit.clone(),
        target: embedded.target.clone(),
        embedded_path: embedded.embedded_path.clone(),
        embedded_binary_sha256: embedded.binary_sha256.clone(),
        embedded_binary_size_bytes: embedded.binary_size_bytes,
        selected_binary_sha256: selected.sha256.clone(),
        selected_binary_size_bytes: selected.size_bytes,
        linked_at_ms,
        approved,
        message: if approved {
            "The selected executable is byte-for-byte identical to pulsedag-miner inside the approved archive."
                .into()
        } else {
            "The selected executable does not match pulsedag-miner inside the approved archive. Do not run it."
                .into()
        },
    };
    let mut guard = state
        .provenance
        .lock()
        .map_err(|_| "Miner provenance state is unavailable.".to_string())?;
    if approved {
        *guard = Some(TrustedBinaryProvenance {
            executable_path: selected.path,
            binary_sha256: selected.sha256,
            binary_size_bytes: selected.size_bytes,
            archive_name: release.archive_name,
            archive_sha256: release.sha256,
            release_tag: release.release_tag,
            source_commit: release.source_commit,
            target: embedded.target,
            embedded_path: embedded.embedded_path,
            linked_at_ms,
        });
    } else {
        *guard = None;
    }
    Ok(result)
}

#[tauri::command]
fn start_verified_miner(
    config: MinerLaunchConfig,
    supervisor: State<'_, MinerSupervisor>,
    provenance: State<'_, MinerProvenanceRegistry>,
) -> Result<MinerRuntimeStatus, String> {
    let profile = config.config_profile.trim().to_ascii_lowercase();
    if profile != "private" {
        return Err("The verified miner launch command is reserved for the private profile.".into());
    }
    let binary = validate_miner_binary_path(Path::new(config.executable_path.trim()))?;
    let proof = verify_miner_provenance_for_launch(&provenance, &binary, &profile)?
        .ok_or_else(|| "Approved miner provenance is required for private-profile mining.".to_string())?;
    let parsed = parse_local_rpc_endpoint(&config.node_endpoint)?;
    TcpStream::connect_timeout(&parsed.socket_addr, Duration::from_millis(1_500))
        .map_err(|error| format!("The local node RPC is not reachable for mining: {error}"))?;
    let node_origin = format!("http://{}", parsed.host_header);
    let mut command_config = config.clone();
    command_config.config_profile = "local".into();
    let args = miner_command_args(&command_config, &node_origin)?;

    let mut managed = supervisor
        .process
        .lock()
        .map_err(|_| "Miner process state is unavailable.".to_string())?;
    refresh_miner_process_state(&supervisor, &mut managed);
    if managed.child.is_some() {
        return Err("pulsedag-miner is already running under this desktop session.".into());
    }
    if let Ok(mut telemetry) = supervisor.telemetry.lock() {
        *telemetry = MinerTelemetrySnapshot::default();
    }

    let mut command = Command::new(&binary.path);
    safe_environment(&mut command);
    if let Some(parent) = Path::new(&binary.path).parent() {
        command.current_dir(parent);
    }
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| format!("Unable to start pulsedag-miner: {error}"))?;
    let pid = child.id();
    if let Some(stdout) = child.stdout.take() {
        spawn_miner_log_reader(
            stdout,
            "miner-stdout",
            Arc::clone(&supervisor.logs),
            Arc::clone(&supervisor.sequence),
            Arc::clone(&supervisor.telemetry),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_miner_log_reader(
            stderr,
            "miner-stderr",
            Arc::clone(&supervisor.logs),
            Arc::clone(&supervisor.sequence),
            Arc::clone(&supervisor.telemetry),
        );
    }

    managed.child = Some(child);
    managed.started_at_ms = Some(unix_time_ms());
    managed.last_exit_code = None;
    managed.executable_path = Some(binary.path.clone());
    push_log(
        &supervisor.logs,
        &supervisor.sequence,
        "miner-desktop",
        format!(
            "started pulsedag-miner pid={pid} backend=cpu provenance=approved:{}:{} rpc={} address={} threads={} max_tries={} profile=private",
            proof.release_tag,
            proof.archive_name,
            parsed.socket_addr,
            miner_address_label(&config.miner_address),
            config.threads,
            config.max_tries,
        ),
    );

    thread::sleep(Duration::from_millis(150));
    refresh_miner_process_state(&supervisor, &mut managed);
    if managed.child.is_none() {
        return Err("pulsedag-miner exited immediately. Open the Mining log for the startup error.".into());
    }

    Ok(miner_runtime_status(&managed, &supervisor.telemetry))
}

#[cfg(test)]
mod miner_provenance_tests {
    use super::*;

    #[test]
    fn miner_provenance_archive_layout_is_target_and_format_bound() {
        let linux = miner_provenance_archive_layout(
            "pulsedag-miner-v2.3.0-x86_64-unknown-linux-gnu.tar.gz",
        )
        .expect("linux miner archive layout");
        assert_eq!(linux.binary_name, "pulsedag-miner");
        assert_eq!(linux.allowed_files.len(), 3);
        let windows = miner_provenance_archive_layout(
            "pulsedag-miner-v2.3.0-x86_64-pc-windows-msvc.zip",
        )
        .expect("windows miner archive layout");
        assert_eq!(windows.binary_name, "pulsedag-miner.exe");
        assert!(miner_provenance_archive_layout(
            "pulsedag-miner-v2.3.0-x86_64-pc-windows-msvc.tar.gz"
        )
        .is_err());
        assert!(miner_provenance_archive_layout("pulsedagd-v2.3.0-x86_64-pc-windows-msvc.zip").is_err());
    }

    #[test]
    fn miner_provenance_private_launch_requires_current_binding() {
        let registry = MinerProvenanceRegistry::default();
        let binary = BinaryInfo {
            path: "/tmp/pulsedag-miner".into(),
            file_name: "pulsedag-miner".into(),
            size_bytes: 4,
            sha256: "a".repeat(64),
            executable: true,
        };
        assert!(verify_miner_provenance_for_launch(&registry, &binary, "private").is_err());
        assert!(verify_miner_provenance_for_launch(&registry, &binary, "dev")
            .expect("dev profile")
            .is_none());
    }
}
