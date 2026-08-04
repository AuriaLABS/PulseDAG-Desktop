const MAX_MINER_LOG_ENTRIES: usize = 5_000;
const DEFAULT_MINER_LOG_LIMIT: usize = 250;
const MAX_MINER_LOG_LIMIT: usize = 500;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MinerLaunchConfig {
    executable_path: String,
    node_endpoint: String,
    miner_address: String,
    config_profile: String,
    threads: usize,
    max_tries: u64,
    sleep_ms: u64,
    refresh_before_expiry_ms: u64,
    worker_id: String,
    heartbeat: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct MinerTelemetrySnapshot {
    last_event: Option<String>,
    backend: Option<String>,
    workers: Option<usize>,
    attempts: u64,
    hashes_per_sec: f64,
    templates_received: u64,
    templates_skipped_stale: u64,
    submits_total: u64,
    submits_accepted: u64,
    submits_rejected: u64,
    last_reject_code: Option<String>,
    last_template_height: Option<u64>,
    last_accepted_height: Option<u64>,
    updated_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MinerRuntimeStatus {
    running: bool,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    uptime_seconds: Option<u64>,
    last_exit_code: Option<i32>,
    executable_path: Option<String>,
    telemetry: MinerTelemetrySnapshot,
}

#[derive(Default)]
struct ManagedMinerProcess {
    child: Option<Child>,
    started_at_ms: Option<u64>,
    last_exit_code: Option<i32>,
    executable_path: Option<String>,
}

struct MinerSupervisor {
    process: Mutex<ManagedMinerProcess>,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    sequence: Arc<AtomicU64>,
    telemetry: Arc<Mutex<MinerTelemetrySnapshot>>,
}

impl Default for MinerSupervisor {
    fn default() -> Self {
        Self {
            process: Mutex::new(ManagedMinerProcess::default()),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            telemetry: Arc::new(Mutex::new(MinerTelemetrySnapshot::default())),
        }
    }
}

fn candidate_miner_binary_paths() -> Vec<PathBuf> {
    let file_name = if cfg!(windows) {
        "pulsedag-miner.exe"
    } else {
        "pulsedag-miner"
    };
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os("PULSEDAG_MINER_PATH") {
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

fn validate_miner_binary_path(path: &Path) -> Result<BinaryInfo, String> {
    if path.as_os_str().is_empty() {
        return Err("Select a pulsedag-miner executable first.".into());
    }

    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve miner executable path: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect miner executable: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected miner path is not a regular file.".into());
    }

    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The miner executable file name is not valid UTF-8.".to_string())?
        .to_string();
    let normalized_name = file_name.to_ascii_lowercase();
    if normalized_name != "pulsedag-miner" && normalized_name != "pulsedag-miner.exe" {
        return Err(
            "The selected file must be named pulsedag-miner or pulsedag-miner.exe.".into(),
        );
    }

    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    };
    #[cfg(not(unix))]
    let executable = true;

    if !executable {
        return Err("The selected pulsedag-miner file is not executable.".into());
    }

    Ok(BinaryInfo {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        sha256: sha256_path(&canonical)?,
        executable,
    })
}

fn miner_runtime_status(
    managed: &ManagedMinerProcess,
    telemetry: &Arc<Mutex<MinerTelemetrySnapshot>>,
) -> MinerRuntimeStatus {
    let running = managed.child.is_some();
    let pid = managed.child.as_ref().map(Child::id);
    let uptime_seconds = managed
        .started_at_ms
        .filter(|_| running)
        .map(|started| unix_time_ms().saturating_sub(started) / 1_000);
    let telemetry = telemetry.lock().map(|value| value.clone()).unwrap_or_default();

    MinerRuntimeStatus {
        running,
        pid,
        started_at_ms: managed.started_at_ms,
        uptime_seconds,
        last_exit_code: managed.last_exit_code,
        executable_path: managed.executable_path.clone(),
        telemetry,
    }
}

fn update_miner_telemetry(line: &str, telemetry: &Arc<Mutex<MinerTelemetrySnapshot>>) {
    if !line.starts_with("miner_telemetry ") {
        return;
    }
    let Ok(mut snapshot) = telemetry.lock() else {
        return;
    };

    for token in line.split_whitespace().skip(1) {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        match key {
            "event" => snapshot.last_event = Some(value.to_string()),
            "backend" => snapshot.backend = Some(value.to_string()),
            "workers" => snapshot.workers = value.parse().ok(),
            "attempts" => {
                if let Ok(value) = value.parse() {
                    snapshot.attempts = value;
                }
            }
            "hashes_per_sec" => {
                if let Ok(value) = value.parse() {
                    snapshot.hashes_per_sec = value;
                }
            }
            "templates_received" => {
                if let Ok(value) = value.parse() {
                    snapshot.templates_received = value;
                }
            }
            "templates_skipped_stale" => {
                if let Ok(value) = value.parse() {
                    snapshot.templates_skipped_stale = value;
                }
            }
            "submits_total" => {
                if let Ok(value) = value.parse() {
                    snapshot.submits_total = value;
                }
            }
            "submits_accepted" => {
                if let Ok(value) = value.parse() {
                    snapshot.submits_accepted = value;
                }
            }
            "submits_rejected" => {
                if let Ok(value) = value.parse() {
                    snapshot.submits_rejected = value;
                }
            }
            "last_reject_code" => {
                snapshot.last_reject_code = (value != "-").then(|| value.to_string())
            }
            "last_template_height" => snapshot.last_template_height = value.parse().ok(),
            "last_accepted_height" => snapshot.last_accepted_height = value.parse().ok(),
            _ => {}
        }
    }
    snapshot.updated_at_ms = Some(unix_time_ms());
}

fn spawn_miner_log_reader<R: Read + Send + 'static>(
    reader: R,
    stream: &'static str,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    sequence: Arc<AtomicU64>,
    telemetry: Arc<Mutex<MinerTelemetrySnapshot>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => {
                    update_miner_telemetry(&line, &telemetry);
                    push_log(&logs, &sequence, stream, line);
                    if let Ok(mut queue) = logs.lock() {
                        while queue.len() > MAX_MINER_LOG_ENTRIES {
                            queue.pop_front();
                        }
                    }
                }
                Err(error) => {
                    push_log(
                        &logs,
                        &sequence,
                        "miner-desktop",
                        format!("miner log stream error: {error}"),
                    );
                    break;
                }
            }
        }
    });
}

fn refresh_miner_process_state(
    supervisor: &MinerSupervisor,
    managed: &mut ManagedMinerProcess,
) {
    let exit_code = match managed.child.as_mut() {
        Some(child) => match child.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            Ok(None) => None,
            Err(error) => {
                push_log(
                    &supervisor.logs,
                    &supervisor.sequence,
                    "miner-desktop",
                    format!("failed to inspect miner process: {error}"),
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
            "miner-desktop",
            format!("pulsedag-miner exited with code {code}"),
        );
    }
}

fn validate_miner_text(value: &str, label: &str, maximum: usize, required: bool) -> Result<String, String> {
    let trimmed = value.trim();
    if required && trimmed.is_empty() {
        return Err(format!("{label} is required."));
    }
    if trimmed.len() > maximum || trimmed.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err(format!("{label} must be at most {maximum} characters without whitespace."));
    }
    Ok(trimmed.to_string())
}

fn miner_command_args(config: &MinerLaunchConfig, node_origin: &str) -> Result<Vec<String>, String> {
    let profile = config.config_profile.trim().to_ascii_lowercase();
    if !matches!(profile.as_str(), "dev" | "local" | "private") {
        return Err("Supported configuration profiles are dev, local and private.".into());
    }
    if profile == "private" {
        return Err(
            "Private-profile mining is blocked until pulsedag-miner is linked to an approved release archive. Use dev or local for this integration milestone."
                .into(),
        );
    }
    let address = validate_miner_text(&config.miner_address, "Miner address", 256, true)?;
    let worker_id = validate_miner_text(&config.worker_id, "Worker ID", 128, false)?;
    if !(1..=256).contains(&config.threads) {
        return Err("Miner threads must be between 1 and 256.".into());
    }
    if !(1..=100_000_000).contains(&config.max_tries) {
        return Err("Maximum tries must be between 1 and 100,000,000.".into());
    }
    if !(100..=60_000).contains(&config.sleep_ms) {
        return Err("Miner sleep must be between 100 and 60,000 milliseconds.".into());
    }
    if config.refresh_before_expiry_ms > 60_000 {
        return Err("Template refresh threshold must not exceed 60,000 milliseconds.".into());
    }

    let mut args = vec![
        "--node".into(),
        node_origin.into(),
        "--miner-address".into(),
        address,
        "--backend".into(),
        "cpu".into(),
        "--threads".into(),
        config.threads.to_string(),
        "--max-tries".into(),
        config.max_tries.to_string(),
        "--loop".into(),
        "--sleep-ms".into(),
        config.sleep_ms.to_string(),
        "--refresh-before-expiry-ms".into(),
        config.refresh_before_expiry_ms.to_string(),
    ];
    if !worker_id.is_empty() {
        args.push("--worker-id".into());
        args.push(worker_id);
    }
    args.push(if config.heartbeat {
        "--heartbeat".into()
    } else {
        "--no-heartbeat".into()
    });
    Ok(args)
}

fn miner_address_label(address: &str) -> String {
    let value = address.trim();
    let suffix: String = value.chars().rev().take(8).collect::<String>().chars().rev().collect();
    if value.len() > suffix.len() {
        format!("…{suffix}")
    } else {
        suffix
    }
}

#[tauri::command]
fn discover_miner_binary() -> Option<BinaryInfo> {
    candidate_miner_binary_paths()
        .into_iter()
        .find_map(|candidate| validate_miner_binary_path(&candidate).ok())
}

#[tauri::command]
fn validate_miner_binary(path: String) -> Result<BinaryInfo, String> {
    validate_miner_binary_path(Path::new(path.trim()))
}

#[tauri::command]
fn get_miner_status(state: State<'_, MinerSupervisor>) -> Result<MinerRuntimeStatus, String> {
    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Miner process state is unavailable.".to_string())?;
    refresh_miner_process_state(&state, &mut managed);
    Ok(miner_runtime_status(&managed, &state.telemetry))
}

#[tauri::command]
fn start_miner(
    config: MinerLaunchConfig,
    state: State<'_, MinerSupervisor>,
) -> Result<MinerRuntimeStatus, String> {
    let binary = validate_miner_binary_path(Path::new(config.executable_path.trim()))?;
    let parsed = parse_local_rpc_endpoint(&config.node_endpoint)?;
    TcpStream::connect_timeout(&parsed.socket_addr, Duration::from_millis(1_500))
        .map_err(|error| format!("The local node RPC is not reachable for mining: {error}"))?;
    let node_origin = format!("http://{}", parsed.host_header);
    let args = miner_command_args(&config, &node_origin)?;

    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Miner process state is unavailable.".to_string())?;
    refresh_miner_process_state(&state, &mut managed);
    if managed.child.is_some() {
        return Err("pulsedag-miner is already running under this desktop session.".into());
    }
    if let Ok(mut telemetry) = state.telemetry.lock() {
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
            Arc::clone(&state.logs),
            Arc::clone(&state.sequence),
            Arc::clone(&state.telemetry),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_miner_log_reader(
            stderr,
            "miner-stderr",
            Arc::clone(&state.logs),
            Arc::clone(&state.sequence),
            Arc::clone(&state.telemetry),
        );
    }

    managed.child = Some(child);
    managed.started_at_ms = Some(unix_time_ms());
    managed.last_exit_code = None;
    managed.executable_path = Some(binary.path.clone());
    push_log(
        &state.logs,
        &state.sequence,
        "miner-desktop",
        format!(
            "started pulsedag-miner pid={pid} backend=cpu rpc={} address={} threads={} max_tries={} profile={}",
            parsed.socket_addr,
            miner_address_label(&config.miner_address),
            config.threads,
            config.max_tries,
            config.config_profile.trim().to_ascii_lowercase(),
        ),
    );

    thread::sleep(Duration::from_millis(150));
    refresh_miner_process_state(&state, &mut managed);
    if managed.child.is_none() {
        return Err("pulsedag-miner exited immediately. Open the Mining log for the startup error.".into());
    }

    Ok(miner_runtime_status(&managed, &state.telemetry))
}

#[tauri::command]
fn stop_miner(state: State<'_, MinerSupervisor>) -> Result<MinerRuntimeStatus, String> {
    let mut managed = state
        .process
        .lock()
        .map_err(|_| "Miner process state is unavailable.".to_string())?;
    refresh_miner_process_state(&state, &mut managed);
    let Some(child) = managed.child.as_mut() else {
        return Ok(miner_runtime_status(&managed, &state.telemetry));
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
                    .map_err(|error| format!("Failed to force-stop pulsedag-miner: {error}"))?;
                let status = child
                    .wait()
                    .map_err(|error| format!("Failed waiting for pulsedag-miner to exit: {error}"))?;
                break status.code().unwrap_or(-1);
            }
            Err(error) => {
                return Err(format!("Failed waiting for pulsedag-miner to exit: {error}"))
            }
        }
    };

    managed.child = None;
    managed.started_at_ms = None;
    managed.last_exit_code = Some(exit_code);
    push_log(
        &state.logs,
        &state.sequence,
        "miner-desktop",
        format!("stopped pulsedag-miner pid={pid} exit_code={exit_code}"),
    );
    Ok(miner_runtime_status(&managed, &state.telemetry))
}

#[tauri::command]
fn get_miner_logs(
    after: Option<u64>,
    limit: Option<usize>,
    state: State<'_, MinerSupervisor>,
) -> LogBatch {
    let cursor = after.unwrap_or_default();
    let limit = limit
        .unwrap_or(DEFAULT_MINER_LOG_LIMIT)
        .clamp(1, MAX_MINER_LOG_LIMIT);
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
    LogBatch {
        entries,
        next_cursor,
    }
}

#[tauri::command]
fn clear_miner_logs(state: State<'_, MinerSupervisor>) -> Result<(), String> {
    state
        .logs
        .lock()
        .map_err(|_| "Miner log state is unavailable.".to_string())?
        .clear();
    Ok(())
}

#[cfg(test)]
mod miner_tests {
    use super::*;

    fn sample_config() -> MinerLaunchConfig {
        MinerLaunchConfig {
            executable_path: "/tmp/pulsedag-miner".into(),
            node_endpoint: "http://127.0.0.1:8080".into(),
            miner_address: "pulsedag:test-address".into(),
            config_profile: "dev".into(),
            threads: 4,
            max_tries: 500_000,
            sleep_ms: 1_000,
            refresh_before_expiry_ms: 1_000,
            worker_id: "desktop-worker".into(),
            heartbeat: true,
        }
    }

    #[test]
    fn miner_telemetry_line_is_parsed() {
        let telemetry = Arc::new(Mutex::new(MinerTelemetrySnapshot::default()));
        update_miner_telemetry(
            "miner_telemetry event=submit_accepted backend=cpu workers=4 attempts=1200 hashes_per_sec=9876.50 templates_received=3 templates_skipped_stale=1 submits_total=2 submits_accepted=1 submits_rejected=1 last_reject_code=- last_template_height=20 last_accepted_height=19",
            &telemetry,
        );
        let snapshot = telemetry.lock().unwrap().clone();
        assert_eq!(snapshot.last_event.as_deref(), Some("submit_accepted"));
        assert_eq!(snapshot.backend.as_deref(), Some("cpu"));
        assert_eq!(snapshot.workers, Some(4));
        assert_eq!(snapshot.attempts, 1_200);
        assert_eq!(snapshot.submits_accepted, 1);
        assert_eq!(snapshot.last_accepted_height, Some(19));
        assert!(snapshot.updated_at_ms.is_some());
    }

    #[test]
    fn miner_command_is_cpu_loop_and_bounded() {
        let args = miner_command_args(&sample_config(), "http://127.0.0.1:8080")
            .expect("valid miner command");
        assert!(args.windows(2).any(|pair| pair == ["--backend", "cpu"]));
        assert!(args.iter().any(|arg| arg == "--loop"));
        assert!(args.iter().any(|arg| arg == "--heartbeat"));
        assert!(!args.iter().any(|arg| arg.contains(';')));
    }

    #[test]
    fn miner_config_rejects_private_and_unbounded_values() {
        let mut config = sample_config();
        config.config_profile = "private".into();
        assert!(miner_command_args(&config, "http://127.0.0.1:8080").is_err());
        config.config_profile = "dev".into();
        config.threads = 0;
        assert!(miner_command_args(&config, "http://127.0.0.1:8080").is_err());
        config.threads = 4;
        config.miner_address = "bad address".into();
        assert!(miner_command_args(&config, "http://127.0.0.1:8080").is_err());
    }

    #[test]
    fn miner_address_log_label_is_redacted() {
        assert_eq!(miner_address_label("pulsedag:abcdefgh12345678"), "…12345678");
        assert_eq!(miner_address_label("short"), "short");
    }
}
