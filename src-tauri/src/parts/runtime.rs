const DEFAULT_LOG_WINDOW: usize = 2_000;
const MIN_LOG_WINDOW: usize = 250;

fn normalize_log_window(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_LOG_WINDOW)
        .clamp(MIN_LOG_WINDOW, MAX_LOG_ENTRIES)
}

fn log_tail(queue: &VecDeque<LogEntry>, limit: usize) -> Vec<LogEntry> {
    let start = queue.len().saturating_sub(limit);
    queue.iter().skip(start).cloned().collect()
}

fn normalize_windows_verbatim_path(path: &str) -> Result<String, String> {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        if rest.is_empty() {
            return Err("The Windows UNC data directory is incomplete.".into());
        }
        return Ok(format!(r"\\{rest}"));
    }

    if let Some(rest) = path.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        let is_drive_path = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/');
        if is_drive_path {
            return Ok(rest.to_string());
        }
        return Err(
            "This Windows data directory uses an unsupported verbatim path. Choose a normal drive or UNC directory."
                .into(),
        );
    }

    Ok(path.to_string())
}

#[cfg(windows)]
fn normalize_data_directory_for_node(path: PathBuf) -> Result<PathBuf, String> {
    let path_text = path.to_str().ok_or_else(|| {
        "The Windows data directory cannot be represented safely for pulsedagd.".to_string()
    })?;
    normalize_windows_verbatim_path(path_text).map(PathBuf::from)
}

#[cfg(not(windows))]
fn normalize_data_directory_for_node(path: PathBuf) -> Result<PathBuf, String> {
    Ok(path)
}

fn prepare_data_directory(path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(path).map_err(|error| format!("Cannot create data directory: {error}"))?;
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve data directory: {error}"))?;
    normalize_data_directory_for_node(canonical)
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
fn start_node(
    config: NodeLaunchConfig,
    state: State<'_, NodeSupervisor>,
) -> Result<NodeRuntimeStatus, String> {
    let binary = validate_binary_path(Path::new(config.executable_path.trim()))?;
    let rpc = parse_local_rpc_endpoint(&config.rpc_endpoint)?;
    let profile = config.config_profile.trim().to_ascii_lowercase();
    if !matches!(profile.as_str(), "dev" | "local" | "private") {
        return Err("Supported configuration profiles are dev, local and private.".into());
    }
    let provenance = verify_binary_provenance_for_launch(&state, &binary, &profile)?;
    if profile == "private" {
        let proof = provenance.as_ref().ok_or_else(|| {
            "PulseDAG v2.4.0 private mode requires final Task31 node provenance in this desktop session."
                .to_string()
        })?;
        if !is_final_v2_4_node_provenance(proof) {
            return Err(
                "Private mode requires pulsedagd from the final PulseDAG v2.4.0 Task31 release. A v2.3 or provisional proof cannot authorize this runtime."
                    .into(),
            );
        }
    }

    let configured_data_directory = PathBuf::from(config.data_directory.trim());
    if configured_data_directory.as_os_str().is_empty() {
        return Err("Choose a persistent data directory before starting the node.".into());
    }
    let data_directory = prepare_data_directory(&configured_data_directory)?;
    if profile == "private" {
        ensure_v2_4_private_state_boundary(&data_directory)?;
        ensure_v2_4_final_private_state_binding(&data_directory)?;
    }
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
            if profile == "private" {
                "private_operator"
            } else {
                "local_dev"
            },
        );
    if profile == "private" {
        apply_v2_4_private_environment(&mut command);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("Unable to start pulsedagd: {error}"))?;
    let pid = child.id();
    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(
            stdout,
            "stdout",
            Arc::clone(&state.logs),
            Arc::clone(&state.sequence),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(
            stderr,
            "stderr",
            Arc::clone(&state.logs),
            Arc::clone(&state.sequence),
        );
    }

    let started_at_ms = unix_time_ms();
    managed.child = Some(child);
    managed.started_at_ms = Some(started_at_ms);
    managed.last_exit_code = None;
    managed.executable_path = Some(binary.path.clone());
    let provenance_label = provenance
        .as_ref()
        .map(|proof| format!("approved:{}:{}", proof.release_tag, proof.archive_name))
        .unwrap_or_else(|| "unverified-development".into());
    let identity_label = if profile == "private" {
        format!(
            " network={} chain={} protocol={} single_node=true p2p=false public_testnet=false contracts=false",
            V2_4_PRIVATE_NETWORK_PROFILE,
            V2_4_PRIVATE_CHAIN_ID,
            V2_4_PRIVATE_PROTOCOL_CONSENSUS_MODE,
        )
    } else {
        String::new()
    };
    push_log(
        &state.logs,
        &state.sequence,
        "desktop",
        format!(
            "started pulsedagd pid={pid} profile={profile} provenance={provenance_label} rpc={} data={}{}",
            rpc.socket_addr,
            data_directory.display(),
            identity_label,
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
        Err(format!(
            "Failed to send SIGTERM: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(not(unix))]
fn request_graceful_stop(child: &mut Child) -> Result<(), String> {
    child
        .kill()
        .map_err(|error| format!("Failed to stop pulsedagd: {error}"))
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
    let mut stream = match TcpStream::connect_timeout(
        &parsed.socket_addr,
        Duration::from_millis(1_500),
    ) {
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
        .unwrap_or(if reachable {
            "healthy"
        } else {
            "unexpected response"
        });

    Ok(RpcHealth {
        reachable,
        status_code,
        latency_ms: started.elapsed().as_millis(),
        message: body.chars().take(240).collect(),
    })
}

#[tauri::command]
fn get_node_logs(
    after: Option<u64>,
    limit: Option<usize>,
    state: State<'_, NodeSupervisor>,
) -> LogBatch {
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
    LogBatch {
        entries,
        next_cursor,
    }
}

#[tauri::command]
fn get_node_log_tail(limit: Option<usize>, state: State<'_, NodeSupervisor>) -> LogBatch {
    let limit = normalize_log_window(limit);
    let entries = state
        .logs
        .lock()
        .map(|queue| log_tail(&queue, limit))
        .unwrap_or_default();
    let next_cursor = entries.last().map(|entry| entry.sequence).unwrap_or_default();
    LogBatch {
        entries,
        next_cursor,
    }
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

#[cfg(test)]
mod runtime_path_tests {
    use super::*;

    #[test]
    fn runtime_windows_drive_path_drops_verbatim_prefix() {
        assert_eq!(
            normalize_windows_verbatim_path(r"\\?\C:\PulseDAG\data").unwrap(),
            r"C:\PulseDAG\data"
        );
    }

    #[test]
    fn runtime_windows_unc_path_drops_verbatim_prefix() {
        assert_eq!(
            normalize_windows_verbatim_path(r"\\?\UNC\server\share\PulseDAG").unwrap(),
            r"\\server\share\PulseDAG"
        );
    }

    #[test]
    fn runtime_windows_normal_path_is_unchanged() {
        assert_eq!(
            normalize_windows_verbatim_path(r"C:\PulseDAG\data").unwrap(),
            r"C:\PulseDAG\data"
        );
    }

    #[test]
    fn runtime_windows_unsupported_verbatim_path_is_rejected() {
        let error = normalize_windows_verbatim_path(r"\\?\Volume{1234}\PulseDAG")
            .expect_err("volume GUID paths must not be passed to pulsedagd");
        assert!(error.contains("unsupported verbatim path"));
    }
}
