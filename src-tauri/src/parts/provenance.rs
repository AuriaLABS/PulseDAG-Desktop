use flate2::read::GzDecoder;
use std::collections::HashSet;
use std::io::{Seek, SeekFrom};
use zip::ZipArchive;

const MAX_PROVENANCE_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_PROVENANCE_BINARY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PROVENANCE_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PROVENANCE_ENTRIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProvenanceArchiveKind {
    TarGz,
    Zip,
}

#[derive(Debug, Clone)]
struct ProvenanceArchiveLayout {
    base_name: String,
    target: String,
    binary_name: String,
    binary_path: PathBuf,
    allowed_files: HashSet<PathBuf>,
    kind: ProvenanceArchiveKind,
}

#[derive(Debug)]
struct EmbeddedBinaryEvidence {
    target: String,
    embedded_path: String,
    archive_sha256: String,
    binary_sha256: String,
    binary_size_bytes: u64,
}

fn supported_host_release_target() -> Result<&'static str, String> {
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
    Err("PulseDAG v2.3.0 does not publish an approved binary for this desktop target.".into())
}

fn provenance_archive_layout(file_name: &str) -> Result<ProvenanceArchiveLayout, String> {
    let (base_name, kind) = if let Some(base) = file_name.strip_suffix(".tar.gz") {
        (base.to_string(), ProvenanceArchiveKind::TarGz)
    } else if let Some(base) = file_name.strip_suffix(".zip") {
        (base.to_string(), ProvenanceArchiveKind::Zip)
    } else {
        return Err("The approved release archive must be a .tar.gz or .zip file.".into());
    };
    let target = base_name
        .strip_prefix("pulsedagd-v2.3.0-")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "The release archive name does not identify PulseDAG v2.3.0.".to_string())?
        .to_string();
    if !matches!(
        target.as_str(),
        "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "x86_64-apple-darwin"
    ) {
        return Err("The release archive target is not approved for PulseDAG v2.3.0.".into());
    }
    let windows_target = target == "x86_64-pc-windows-msvc";
    match (kind, windows_target) {
        (ProvenanceArchiveKind::Zip, true) | (ProvenanceArchiveKind::TarGz, false) => {}
        _ => return Err("The release archive format does not match its declared target.".into()),
    }
    let binary_name = if windows_target { "pulsedagd.exe" } else { "pulsedagd" }.to_string();
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

fn safe_provenance_entry_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, std::path::Component::Normal(_))
        })
}

fn hash_reader_limited<R: Read>(reader: &mut R, maximum: u64, label: &str) -> Result<(String, u64), String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Cannot read {label}: {error}"))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > maximum {
            return Err(format!("{label} exceeds the configured safety limit."));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

fn validate_archive_file_set(
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
            "The release archive has an unexpected file set{}.",
            if missing.is_empty() {
                String::new()
            } else {
                format!("; missing {}", missing.join(", "))
            }
        ));
    }
    Ok(())
}

fn inspect_zip_binary(
    file: File,
    layout: &ProvenanceArchiveLayout,
    archive_sha256: String,
) -> Result<EmbeddedBinaryEvidence, String> {
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("Cannot open the approved ZIP archive: {error}"))?;
    if archive.len() > MAX_PROVENANCE_ENTRIES {
        return Err("The release ZIP contains too many entries.".into());
    }
    let mut seen_files = HashSet::new();
    let mut binary_evidence = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Cannot inspect ZIP entry {index}: {error}"))?;
        let path = entry
            .enclosed_name()
            .ok_or_else(|| "The release ZIP contains an unsafe path.".to_string())?
            .to_path_buf();
        if !safe_provenance_entry_path(&path) {
            return Err("The release ZIP contains an unsafe path.".into());
        }
        if entry.is_dir() {
            if path != PathBuf::from(&layout.base_name) {
                return Err("The release ZIP contains an unexpected directory.".into());
            }
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("The release ZIP contains a symbolic link.".into());
        }
        if !layout.allowed_files.contains(&path) || !seen_files.insert(path.clone()) {
            return Err("The release ZIP contains an unexpected or duplicate file.".into());
        }
        let declared_size = entry.size();
        if path == layout.binary_path {
            if declared_size > MAX_PROVENANCE_BINARY_BYTES {
                return Err("The embedded pulsedagd binary exceeds the safety limit.".into());
            }
            let (binary_sha256, binary_size_bytes) = hash_reader_limited(
                &mut entry,
                MAX_PROVENANCE_BINARY_BYTES,
                "the embedded pulsedagd binary",
            )?;
            if binary_size_bytes != declared_size {
                return Err("The embedded pulsedagd size does not match the ZIP metadata.".into());
            }
            binary_evidence = Some(EmbeddedBinaryEvidence {
                target: layout.target.clone(),
                embedded_path: path.display().to_string(),
                archive_sha256: archive_sha256.clone(),
                binary_sha256,
                binary_size_bytes,
            });
        } else if declared_size > MAX_PROVENANCE_DOCUMENT_BYTES {
            return Err("A release document exceeds the safety limit.".into());
        }
    }
    validate_archive_file_set(layout, &seen_files)?;
    binary_evidence.ok_or_else(|| "The approved ZIP does not contain pulsedagd.".into())
}

fn inspect_tar_binary(
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
        .map_err(|error| format!("Cannot open the approved TAR.GZ archive: {error}"))?;
    for entry in entries {
        entry_count += 1;
        if entry_count > MAX_PROVENANCE_ENTRIES {
            return Err("The release TAR.GZ contains too many entries.".into());
        }
        let mut entry = entry.map_err(|error| format!("Cannot inspect TAR entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("Cannot decode a TAR entry path: {error}"))?
            .into_owned();
        if !safe_provenance_entry_path(&path) {
            return Err("The release TAR.GZ contains an unsafe path.".into());
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            if path != PathBuf::from(&layout.base_name) {
                return Err("The release TAR.GZ contains an unexpected directory.".into());
            }
            continue;
        }
        if !entry_type.is_file() {
            return Err("The release TAR.GZ contains a link or unsupported entry type.".into());
        }
        if !layout.allowed_files.contains(&path) || !seen_files.insert(path.clone()) {
            return Err("The release TAR.GZ contains an unexpected or duplicate file.".into());
        }
        let declared_size = entry.size();
        if path == layout.binary_path {
            if declared_size > MAX_PROVENANCE_BINARY_BYTES {
                return Err("The embedded pulsedagd binary exceeds the safety limit.".into());
            }
            let (binary_sha256, binary_size_bytes) = hash_reader_limited(
                &mut entry,
                MAX_PROVENANCE_BINARY_BYTES,
                "the embedded pulsedagd binary",
            )?;
            if binary_size_bytes != declared_size {
                return Err("The embedded pulsedagd size does not match the TAR metadata.".into());
            }
            binary_evidence = Some(EmbeddedBinaryEvidence {
                target: layout.target.clone(),
                embedded_path: path.display().to_string(),
                archive_sha256: archive_sha256.clone(),
                binary_sha256,
                binary_size_bytes,
            });
        } else if declared_size > MAX_PROVENANCE_DOCUMENT_BYTES {
            return Err("A release document exceeds the safety limit.".into());
        }
    }
    validate_archive_file_set(layout, &seen_files)?;
    binary_evidence.ok_or_else(|| "The approved TAR.GZ does not contain pulsedagd.".into())
}

fn inspect_embedded_binary(path: &Path) -> Result<EmbeddedBinaryEvidence, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve the release archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect the release archive: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_PROVENANCE_ARCHIVE_BYTES {
        return Err("The release archive is not a regular file within the safety limit.".into());
    }
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The release archive name is not valid UTF-8.".to_string())?;
    let layout = provenance_archive_layout(file_name)?;
    let host_target = supported_host_release_target()?;
    if layout.target != host_target {
        return Err(format!(
            "The archive target {} does not match this desktop target {host_target}.",
            layout.target
        ));
    }
    let mut file = File::open(&canonical)
        .map_err(|error| format!("Cannot open the release archive: {error}"))?;
    let (archive_sha256, archive_size_bytes) = hash_reader_limited(
        &mut file,
        MAX_PROVENANCE_ARCHIVE_BYTES,
        "the release archive",
    )?;
    if archive_size_bytes != metadata.len() {
        return Err("The release archive changed while it was being inspected.".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Cannot rewind the release archive: {error}"))?;
    match layout.kind {
        ProvenanceArchiveKind::Zip => inspect_zip_binary(file, &layout, archive_sha256),
        ProvenanceArchiveKind::TarGz => inspect_tar_binary(file, &layout, archive_sha256),
    }
}

fn public_binary_provenance(proof: &TrustedBinaryProvenance) -> BinaryProvenance {
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
        message: "The selected executable matches the pulsedagd bytes inside the approved release archive."
            .into(),
    }
}

fn diagnostic_binary_provenance(state: &NodeSupervisor) -> Option<DiagnosticProvenance> {
    state
        .provenance
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().cloned())
        .map(|proof| DiagnosticProvenance {
            archive_name: proof.archive_name,
            archive_sha256: proof.archive_sha256,
            release_tag: proof.release_tag,
            source_commit: proof.source_commit,
            target: proof.target,
            embedded_binary_sha256: proof.binary_sha256,
            binary_size_bytes: proof.binary_size_bytes,
            linked_at_ms: proof.linked_at_ms,
        })
}

fn verify_binary_provenance_for_launch(
    state: &NodeSupervisor,
    binary: &BinaryInfo,
    profile: &str,
) -> Result<Option<TrustedBinaryProvenance>, String> {
    let mut guard = state
        .provenance
        .lock()
        .map_err(|_| "Binary provenance state is unavailable.".to_string())?;
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
            "The private profile requires pulsedagd to be linked to an approved release archive in this desktop session."
                .into(),
        );
    }
    Ok(None)
}

#[tauri::command]
fn get_binary_provenance(
    state: State<'_, NodeSupervisor>,
) -> Result<Option<BinaryProvenance>, String> {
    let proof = state
        .provenance
        .lock()
        .map_err(|_| "Binary provenance state is unavailable.".to_string())?
        .as_ref()
        .cloned();
    let Some(proof) = proof else {
        return Ok(None);
    };
    match validate_binary_path(Path::new(&proof.executable_path)) {
        Ok(binary)
            if binary.sha256.eq_ignore_ascii_case(&proof.binary_sha256)
                && binary.size_bytes == proof.binary_size_bytes =>
        {
            Ok(Some(public_binary_provenance(&proof)))
        }
        _ => {
            state
                .provenance
                .lock()
                .map_err(|_| "Binary provenance state is unavailable.".to_string())?
                .take();
            Ok(None)
        }
    }
}

#[tauri::command]
async fn bind_binary_to_verified_archive(
    archive_path: String,
    executable_path: String,
    state: State<'_, NodeSupervisor>,
) -> Result<BinaryProvenance, String> {
    let release = verify_approved_release_archive(archive_path.clone()).await?;
    if !release.approved {
        return Err(release.message);
    }
    let task_archive = archive_path.clone();
    let task_binary = executable_path.clone();
    let (embedded, selected) = tauri::async_runtime::spawn_blocking(move || {
        let embedded = inspect_embedded_binary(Path::new(task_archive.trim()))?;
        let selected = validate_binary_path(Path::new(task_binary.trim()))?;
        Ok::<_, String>((embedded, selected))
    })
    .await
    .map_err(|error| format!("Binary provenance task failed: {error}"))??;
    if !embedded.archive_sha256.eq_ignore_ascii_case(&release.sha256) {
        return Err("The release archive changed after its GitHub digest was verified.".into());
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
            "The selected executable is byte-for-byte identical to pulsedagd inside the approved archive."
                .into()
        } else {
            "The selected executable does not match pulsedagd inside the approved archive. Do not run it."
                .into()
        },
    };
    let mut guard = state
        .provenance
        .lock()
        .map_err(|_| "Binary provenance state is unavailable.".to_string())?;
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

#[cfg(test)]
mod provenance_tests {
    use super::*;

    #[test]
    fn provenance_archive_layout_is_target_and_format_bound() {
        let linux = provenance_archive_layout(
            "pulsedagd-v2.3.0-x86_64-unknown-linux-gnu.tar.gz",
        )
        .expect("linux archive layout");
        assert_eq!(linux.binary_name, "pulsedagd");
        assert_eq!(linux.allowed_files.len(), 3);
        let windows = provenance_archive_layout(
            "pulsedagd-v2.3.0-x86_64-pc-windows-msvc.zip",
        )
        .expect("windows archive layout");
        assert_eq!(windows.binary_name, "pulsedagd.exe");
        assert!(provenance_archive_layout(
            "pulsedagd-v2.3.0-x86_64-pc-windows-msvc.tar.gz"
        )
        .is_err());
        assert!(provenance_archive_layout("pulsedagd-v2.3.0-unknown.zip").is_err());
    }

    #[test]
    fn provenance_entry_paths_reject_escape_and_aliases() {
        assert!(safe_provenance_entry_path(Path::new(
            "pulsedagd-v2.3.0-x86_64-unknown-linux-gnu/pulsedagd"
        )));
        assert!(!safe_provenance_entry_path(Path::new("../pulsedagd")));
        assert!(!safe_provenance_entry_path(Path::new("./pulsedagd")));
        assert!(!safe_provenance_entry_path(Path::new("/tmp/pulsedagd")));
    }

    #[test]
    fn provenance_private_launch_requires_current_binding() {
        let supervisor = NodeSupervisor::default();
        let binary = BinaryInfo {
            path: "/tmp/pulsedagd".into(),
            file_name: "pulsedagd".into(),
            size_bytes: 4,
            sha256: "a".repeat(64),
            executable: true,
        };
        assert!(verify_binary_provenance_for_launch(&supervisor, &binary, "private").is_err());
        assert!(verify_binary_provenance_for_launch(&supervisor, &binary, "dev")
            .expect("dev profile")
            .is_none());
    }
}
