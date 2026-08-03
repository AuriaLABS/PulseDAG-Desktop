#[tauri::command]
fn export_diagnostics(
    output_path: String,
    config: NodeLaunchConfig,
    rpc_health: RpcHealth,
    app: AppHandle,
    state: State<'_, NodeSupervisor>,
) -> Result<DiagnosticExportResult, String> {
    let output_path = safe_export_path(&output_path)?;
    let runtime = {
        let mut managed = state
            .process
            .lock()
            .map_err(|_| "Node process state is unavailable.".to_string())?;
        refresh_process_state(&state, &mut managed);
        runtime_status(&managed)
    };
    let binary_info = if config.executable_path.trim().is_empty() {
        None
    } else {
        validate_binary_path(Path::new(config.executable_path.trim())).ok()
    };
    let executable_file = binary_info.as_ref().map(|info| info.file_name.clone());
    let binary = binary_info.map(|info| DiagnosticBinary {
        file_name: info.file_name,
        size_bytes: info.size_bytes,
        sha256: info.sha256,
        executable: info.executable,
    });
    let provenance = diagnostic_binary_provenance(&state);
    let safe_rpc_endpoint = parse_local_rpc_endpoint(&config.rpc_endpoint)
        .map(|parsed| format!("http://{}", parsed.host_header))
        .unwrap_or_else(|_| "<invalid-or-redacted>".into());
    let safe_profile = match config.config_profile.trim().to_ascii_lowercase().as_str() {
        "dev" => "dev",
        "local" => "local",
        "private" => "private",
        _ => "<invalid>",
    }
    .to_string();
    let redactions = redaction_values(&config);
    let logs = state
        .logs
        .lock()
        .map_err(|_| "Log state is unavailable.".to_string())?
        .iter()
        .cloned()
        .map(|entry| LogEntry {
            message: redact_text(&entry.message, &redactions),
            ..entry
        })
        .collect::<Vec<_>>();
    let bundle = DiagnosticBundle {
        schema_version: 2,
        desktop: DiagnosticDesktop {
            app_version: app.package_info().version.to_string(),
            platform: env::consts::OS.to_string(),
            generated_at_ms: unix_time_ms(),
        },
        config: DiagnosticConfig {
            executable_file,
            rpc_endpoint: safe_rpc_endpoint,
            data_directory: "<redacted-path>".into(),
            config_profile: safe_profile,
        },
        binary,
        provenance,
        runtime: NodeRuntimeStatus {
            executable_path: runtime
                .executable_path
                .as_deref()
                .map(|_| "<redacted-path>".to_string()),
            ..runtime
        },
        rpc_health,
        logs,
        redactions_applied: vec![
            "node executable path".into(),
            "data directory".into(),
            "home directory".into(),
        ],
    };
    let encoded = serde_json::to_vec_pretty(&bundle)
        .map_err(|error| format!("Cannot serialize diagnostic bundle: {error}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&output_path)
        .map_err(|error| format!("Cannot create diagnostic bundle: {error}"))?;
    file.write_all(&encoded)
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|error| format!("Cannot write diagnostic bundle: {error}"))?;

    let canonical = fs::canonicalize(&output_path).unwrap_or(output_path);
    push_log(
        &state.logs,
        &state.sequence,
        "desktop",
        format!(
            "exported redacted diagnostic bundle file={}",
            canonical
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("diagnostics.json")
        ),
    );
    Ok(DiagnosticExportResult {
        path: canonical.to_string_lossy().into_owned(),
        bytes_written: encoded.len() as u64 + 1,
        log_entries: bundle.logs.len(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(NodeSupervisor::default())
        .invoke_handler(tauri::generate_handler![
            get_desktop_status,
            discover_node_binary,
            validate_node_binary,
            verify_approved_release_archive,
            bind_binary_to_verified_archive,
            get_binary_provenance,
            get_node_status,
            start_node,
            stop_node,
            check_rpc_health,
            get_node_observability,
            get_block_detail,
            get_transaction_detail,
            get_node_logs,
            clear_node_logs,
            export_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PulseDAG Desktop");
}
