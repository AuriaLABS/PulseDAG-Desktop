use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    env,
    fs::{self, File},
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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

    let mut file = File::open(&canonical)
        .map_err(|error| format!("Cannot read executable for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Cannot hash executable: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let sha256 = format!("{:x}", hasher.finalize());

    Ok(BinaryInfo {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        sha256,
        executable,
    })
}

fn candidate_binary_paths() -> Vec<PathBuf> {
    let file_name = if cfg!(windows) { "pulsedagd.exe" } else { "pulsedagd" };
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os("PULSEDAGD_PATH") {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join(file_name));
        }
    }
    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join(file_name));
        candidates.push(current_dir.join("bin").join(file_name));
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            candidates.push(directory.join(file_name));
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn parse_local_rpc_endpoint(raw: &str) -> Result<ParsedRpcEndpoint, String> {
    let url = Url::parse(raw.trim()).map_err(|error| format!("Invalid RPC endpoint: {error}"))?;
    if url.scheme() != "http" {
        return Err("RPC must use plain HTTP on loopback; HTTPS and remote endpoints are not accepted.".into());
    }
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("RPC endpoint must not contain credentials, query parameters or fragments.".into());
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err("Use the RPC origin only, for example http://127.0.0.1:8080.".into());
    }

    let port = url.port_or_known_default().ok_or_else(|| "RPC endpoint must include a port.".to_string())?;
    let (socket_addr, host_header) = match url.host() {
        Some(Host::Ipv4(address)) if address.is_loopback() => {
            (SocketAddr::new(IpAddr::V4(address), port), format!("{address}:{port}"))
        }
        Some(Host::Ipv6(address)) if address.is_loopback() => {
            (SocketAddr::new(IpAddr::V6(address), port), format!("[{address}]:{port}"))
        }
        Some(Host::Domain(domain)) if domain.eq_ignore_ascii_case("localhost") => {
            let address = std::net::Ipv4Addr::LOCALHOST;
            (SocketAddr::new(IpAddr::V4(address), port), format!("localhost:{port}"))
        }
        _ => return Err("RPC must resolve to localhost, 127.0.0.1 or ::1.".into()),
    };

    Ok(ParsedRpcEndpoint { socket_addr, host_header })
}

fn safe_environment(command: &mut Command) {
    command.env_clear();
    for (key, value) in env::vars_os() {
        let normalized = key.to_string_lossy().to_ascii_uppercase();
        if !normalized.starts_with("PULSEDAG_") {
            command.env(key, value);
        }
    }
}

fn runtime_status(managed: &ManagedProcess) -> NodeRuntimeStatus {
    let running = managed.child.is_some();
    let pid = managed.child.as_ref().map(Child::id);
    let uptime_seconds = managed
        .started_at_ms
        .filter(|_| running)
        .map(|started| unix_time_ms().saturating_sub(started) / 1_000);

    NodeRuntimeStatus {
        running,
        pid,
        started_at_ms: managed.started_at_ms,
        uptime_seconds,
        last_exit_code: managed.last_exit_code,
        executable_path: managed.executable_path.clone(),
    }
}

#[tauri::command]
fn get_desktop_status(app: AppHandle, state: State<'_, NodeSupervisor>) -> DesktopStatus {
    let node_running = state
        .process
        .lock()
        .map(|mut managed| {
            refresh_process_state(&state, &mut managed);
            managed.child.is_some()
        })
        .unwrap_or(false);

    DesktopStatus {
        app_version: app.package_info().version.to_string(),
        platform: env::consts::OS.to_string(),
        node_configured: false,
        node_running,
        rpc_reachable: false,
    }
}

#[tauri::command]
fn discover_node_binary() -> Option<BinaryInfo> {
    candidate_binary_paths()
        .into_iter()
        .find_map(|candidate| validate_binary_path(&candidate).ok())
}

#[tauri::command]
fn validate_node_binary(path: String) -> Result<BinaryInfo, String> {
    validate_binary_path(Path::new(path.trim()))
}

#[tauri::command]
fn get_node_status(state: State<'_, NodeSupervisor>) -> Result<NodeRuntimeStatus, String> {
    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Node process state is unavailable.".to_string())?;
    refresh_process_state(&state, &mut managed);
    Ok(runtime_status(&managed))
}

#[tauri::command]
fn start_node(config: NodeLaunchConfig, state: State<'_, NodeSupervisor>) -> Result<NodeRuntimeStatus, String> {
    let binary = validate_binary_path(Path::new(config.executable_path.trim()))?;
    let rpc = parse_local_rpc_endpoint(&config.rpc_endpoint)?;
    let profile = config.config_profile.trim().to_ascii_lowercase();
    if !matches!(profile.as_str(), "dev" | "local" | "private") {
        return Err("Supported configuration profiles are dev, local and private.".into());
    }

    let data_directory = PathBuf::from(config.data_directory.trim());
    if data_directory.as_os_str().is_empty() {
        return Err("Choose a persistent data directory before starting the node.".into());
    }
    fs::create_dir_all(&data_directory)
        .map_err(|error| format!("Cannot create data directory: {error}"))?;
    let data_directory = fs::canonicalize(&data_directory)
        .map_err(|error| format!("Cannot resolve data directory: {error}"))?;
    let rocksdb_path = data_directory.join("rocksdb");
    fs::create_dir_all(&rocksdb_path)
        .map_err(|error| format!("Cannot create RocksDB directory: {error}"))?;
    let identity_path = data_directory.join("identity.key");

    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Node process state is unavailable.".to_string())?;
    refresh_process_state(&state, &mut managed);
    if managed.child.is_some() {
        return Err("pulsedagd is already running under this desktop session.".into());
    }

    let mut command = Command::new(&binary.path);
    safe_environment(&mut command);
    command
        .current_dir(&data_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PULSEDAG_CONFIG_PROFILE", &profile)
        .env("PULSEDAG_RPC_BIND", rpc.socket_addr.to_string())
        .env("PULSEDAG_ROCKSDB_PATH", &rocksdb_path)
        .env("PULSEDAG_P2P_IDENTITY_KEY", &identity_path)
        .env("PULSEDAG_ADMIN_ENABLED", "false")
        .env("PULSEDAG_RPC_CORS_ALLOWLIST", "")
        .env(
            "PULSEDAG_API_PROFILE",
            if profile == "private" { "private_operator" } else { "local_dev" },
        );

    let mut child = command
        .spawn()
        .map_err(|error| format!("Unable to start pulsedagd: {error}"))?;
    let pid = child.id();
    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(stdout, "stdout", Arc::clone(&state.logs), Arc::clone(&state.sequence));
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(stderr, "stderr", Arc::clone(&state.logs), Arc::clone(&state.sequence));
    }

    let started_at_ms = unix_time_ms();
    managed.child = Some(child);
    managed.started_at_ms = Some(started_at_ms);
    managed.last_exit_code = None;
    managed.executable_path = Some(binary.path.clone());
    push_log(
        &state.logs,
        &state.sequence,
        "desktop",
        format!(
            "started pulsedagd pid={pid} profile={profile} rpc={} data={}",
            rpc.socket_addr,
            data_directory.display()
        ),
    );

    thread::sleep(Duration::from_millis(150));
    refresh_process_state(&state, &mut managed);
    if managed.child.is_none() {
        return Err("pulsedagd exited immediately. Open Logs for the startup error.".into());
    }

    Ok(runtime_status(&managed))
}

#[cfg(unix)]
fn request_graceful_stop(child: &mut Child) -> Result<(), String> {
    let result = unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Failed to send SIGTERM: {}", std::io::Error::last_os_error()))
    }
}

#[cfg(not(unix))]
fn request_graceful_stop(child: &mut Child) -> Result<(), String> {
    child.kill().map_err(|error| format!("Failed to stop pulsedagd: {error}"))
}

#[tauri::command]
fn stop_node(state: State<'_, NodeSupervisor>) -> Result<NodeRuntimeStatus, String> {
    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Node process state is unavailable.".to_string())?;
    refresh_process_state(&state, &mut managed);
    let Some(child) = managed.child.as_mut() else {
        return Ok(runtime_status(&managed));
    };

    let pid = child.id();
    request_graceful_stop(child)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                child
                    .kill()
                    .map_err(|error| format!("Failed to force-stop pulsedagd: {error}"))?;
                let status = child
                    .wait()
                    .map_err(|error| format!("Failed waiting for pulsedagd to exit: {error}"))?;
                break status.code().unwrap_or(-1);
            }
            Err(error) => return Err(format!("Failed waiting for pulsedagd to exit: {error}")),
        }
    };

    managed.child = None;
    managed.started_at_ms = None;
    managed.last_exit_code = Some(exit_code);
    push_log(
        &state.logs,
        &state.sequence,
        "desktop",
        format!("stopped pulsedagd pid={pid} exit_code={exit_code}"),
    );
    Ok(runtime_status(&managed))
}

#[tauri::command]
fn check_rpc_health(endpoint: String) -> Result<RpcHealth, String> {
    let parsed = parse_local_rpc_endpoint(&endpoint)?;
    let started = Instant::now();
    let mut stream = match TcpStream::connect_timeout(&parsed.socket_addr, Duration::from_millis(1_500)) {
        Ok(stream) => stream,
        Err(error) => {
            return Ok(RpcHealth {
                reachable: false,
                status_code: None,
                latency_ms: started.elapsed().as_millis(),
                message: format!("RPC connection failed: {error}"),
            })
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_millis(1_500)))
        .map_err(|error| format!("Cannot set RPC read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1_500)))
        .map_err(|error| format!("Cannot set RPC write timeout: {error}"))?;

    let request = format!(
        "GET /health HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        parsed.host_header
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("RPC health request failed: {error}"))?;

    let mut response = String::new();
    stream
        .take(64 * 1024)
        .read_to_string(&mut response)
        .map_err(|error| format!("RPC health response failed: {error}"))?;
    let status_code = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok());
    let reachable = status_code.is_some_and(|code| (200..300).contains(&code));
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.trim())
        .filter(|body| !body.is_empty())
        .unwrap_or(if reachable { "healthy" } else { "unexpected response" });

    Ok(RpcHealth {
        reachable,
        status_code,
        latency_ms: started.elapsed().as_millis(),
        message: body.chars().take(240).collect(),
    })
}

#[tauri::command]
fn get_node_logs(after: Option<u64>, limit: Option<usize>, state: State<'_, NodeSupervisor>) -> LogBatch {
    let cursor = after.unwrap_or_default();
    let limit = limit.unwrap_or(DEFAULT_LOG_LIMIT).clamp(1, MAX_LOG_LIMIT);
    let entries = state
        .logs
        .lock()
        .map(|queue| {
            queue
                .iter()
                .filter(|entry| entry.sequence > cursor)
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let next_cursor = entries.last().map(|entry| entry.sequence).unwrap_or(cursor);
    LogBatch { entries, next_cursor }
}

#[tauri::command]
fn clear_node_logs(state: State<'_, NodeSupervisor>) -> Result<(), String> {
    state
        .logs
        .lock()
        .map_err(|_| "Log state is unavailable.".to_string())?
        .clear();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NodeSupervisor::default())
        .invoke_handler(tauri::generate_handler![
            get_desktop_status,
            discover_node_binary,
            validate_node_binary,
            get_node_status,
            start_node,
            stop_node,
            check_rpc_health,
            get_node_logs,
            clear_node_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PulseDAG Desktop");
}
