use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    env,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, State};
use url::{Host, Url};

const MAX_LOG_ENTRIES: usize = 5_000;
const DEFAULT_LOG_LIMIT: usize = 250;
const MAX_LOG_LIMIT: usize = 500;
const APPROVED_RELEASE_TAG: &str = "v2.3.0";
const APPROVED_RELEASE_COMMIT: &str = "7e43225f01ac05d15e5f1e3f1550d7850bf18cbc";
const APPROVED_RELEASE_API: &str =
    "https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.3.0";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopStatus {
    app_version: String,
    platform: String,
    node_configured: bool,
    node_running: bool,
    rpc_reachable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BinaryInfo {
    path: String,
    file_name: String,
    size_bytes: u64,
    sha256: String,
    executable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeLaunchConfig {
    executable_path: String,
    rpc_endpoint: String,
    data_directory: String,
    config_profile: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeRuntimeStatus {
    running: bool,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    uptime_seconds: Option<u64>,
    last_exit_code: Option<i32>,
    executable_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RpcHealth {
    reachable: bool,
    status_code: Option<u16>,
    latency_ms: u128,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry {
    sequence: u64,
    timestamp_ms: u64,
    stream: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogBatch {
    entries: Vec<LogEntry>,
    next_cursor: u64,
}

#[derive(Debug, Clone)]
struct LocalReleaseArchive {
    path: String,
    file_name: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    digest: Option<String>,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    target_commitish: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseVerification {
    archive_path: String,
    archive_name: String,
    size_bytes: u64,
    sha256: String,
    release_tag: String,
    source_commit: String,
    asset_digest: String,
    approved: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticDesktop {
    app_version: String,
    platform: String,
    generated_at_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticConfig {
    executable_file: Option<String>,
    rpc_endpoint: String,
    data_directory: String,
    config_profile: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticBinary {
    file_name: String,
    size_bytes: u64,
    sha256: String,
    executable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticBundle {
    schema_version: u32,
    desktop: DiagnosticDesktop,
    config: DiagnosticConfig,
    binary: Option<DiagnosticBinary>,
    runtime: NodeRuntimeStatus,
    rpc_health: RpcHealth,
    logs: Vec<LogEntry>,
    redactions_applied: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticExportResult {
    path: String,
    bytes_written: u64,
    log_entries: usize,
}

#[derive(Default)]
struct ManagedProcess {
    child: Option<Child>,
    started_at_ms: Option<u64>,
    last_exit_code: Option<i32>,
    executable_path: Option<String>,
}

struct NodeSupervisor {
    process: Mutex<ManagedProcess>,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    sequence: Arc<AtomicU64>,
}

impl Default for NodeSupervisor {
    fn default() -> Self {
        Self {
            process: Mutex::new(ManagedProcess::default()),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }
}

struct ParsedRpcEndpoint {
    socket_addr: SocketAddr,
    host_header: String,
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn push_log(
    logs: &Arc<Mutex<VecDeque<LogEntry>>>,
    sequence: &Arc<AtomicU64>,
    stream: &str,
    message: impl Into<String>,
) {
    let entry = LogEntry {
        sequence: sequence.fetch_add(1, Ordering::Relaxed) + 1,
        timestamp_ms: unix_time_ms(),
        stream: stream.to_string(),
        message: message.into(),
    };

    if let Ok(mut queue) = logs.lock() {
        queue.push_back(entry);
        while queue.len() > MAX_LOG_ENTRIES {
            queue.pop_front();
        }
    }
}

fn spawn_log_reader<R: Read + Send + 'static>(
    reader: R,
    stream: &'static str,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    sequence: Arc<AtomicU64>,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => push_log(&logs, &sequence, stream, line),
                Err(error) => {
                    push_log(&logs, &sequence, "desktop", format!("log stream error: {error}"));
                    break;
                }
            }
        }
    });
}

fn refresh_process_state(supervisor: &NodeSupervisor, managed: &mut ManagedProcess) {
    let exit_code = match managed.child.as_mut() {
        Some(child) => match child.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            Ok(None) => None,
            Err(error) => {
                push_log(
                    &supervisor.logs,
                    &supervisor.sequence,
                    "desktop",
                    format!("failed to inspect node process: {error}"),
                );
                None
            }
        },
        None => None,
    };

    if let Some(code) = exit_code {
        managed.child = None;
        managed.last_exit_code = Some(code);
        managed.started_at_ms = None;
        push_log(
            &supervisor.logs,
            &supervisor.sequence,
            "desktop",
            format!("pulsedagd exited with code {code}"),
        );
    }
}

fn sha256_path(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("Cannot read file for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Cannot hash file: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_binary_path(path: &Path) -> Result<BinaryInfo, String> {
    if path.as_os_str().is_empty() {
        return Err("Select a pulsedagd executable first.".into());
    }

    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve executable path: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect executable: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected path is not a regular file.".into());
    }

    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The executable file name is not valid UTF-8.".to_string())?
        .to_string();
    let normalized_name = file_name.to_ascii_lowercase();
    if normalized_name != "pulsedagd" && normalized_name != "pulsedagd.exe" {
        return Err("The selected file must be named pulsedagd or pulsedagd.exe.".into());
    }

    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    };
    #[cfg(not(unix))]
    let executable = true;

    if !executable {
        return Err("The selected pulsedagd file is not executable.".into());
    }

    Ok(BinaryInfo {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        sha256: sha256_path(&canonical)?,
        executable,
    })
}

fn inspect_release_archive(path: &Path) -> Result<LocalReleaseArchive, String> {
    if path.as_os_str().is_empty() {
        return Err("Select a PulseDAG release archive first.".into());
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve release archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect release archive: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected release archive is not a regular file.".into());
    }
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The release archive name is not valid UTF-8.".to_string())?
        .to_string();
    let valid_name = file_name.starts_with("pulsedagd-v2.3.0-")
        && (file_name.ends_with(".tar.gz") || file_name.ends_with(".zip"));
    if !valid_name {
        return Err(
            "Select an official pulsedagd-v2.3.0-<target>.tar.gz or .zip archive.".into(),
        );
    }
    Ok(LocalReleaseArchive {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        sha256: sha256_path(&canonical)?,
    })
}

