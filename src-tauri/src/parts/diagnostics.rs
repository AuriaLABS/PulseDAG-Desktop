const DIAGNOSTIC_SCHEMA_VERSION: u32 = 2;

fn collect_diagnostic_logs(
    queue: &VecDeque<LogEntry>,
    config: &NodeLaunchConfig,
    requested_limit: Option<usize>,
) -> Vec<LogEntry> {
    let redactions = redaction_values(config);
    log_tail(queue, normalize_log_window(requested_limit))
        .into_iter()
        .map(|entry| LogEntry {
            message: redact_text(&entry.message, &redactions),
            ..entry
        })
        .collect()
}

#[tauri::command]
fn export_diagnostics(
    output_path: String,
    config: NodeLaunchConfig,
    rpc_health: RpcHealth,
    log_limit: Option<usize>,
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
    let logs = {
        let queue = state
            .logs
            .lock()
            .map_err(|_| "Log state is unavailable.".to_string())?;
        collect_diagnostic_logs(&queue, &config, log_limit)
    };
    let bundle = DiagnosticBundle {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION,
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
            "exported redacted diagnostic bundle file={} log_entries={}",
            canonical
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("diagnostics.json"),
            bundle.logs.len(),
        ),
    );
    Ok(DiagnosticExportResult {
        path: canonical.to_string_lossy().into_owned(),
        bytes_written: encoded.len() as u64 + 1,
        log_entries: bundle.logs.len(),
    })
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    fn sample_bundle() -> DiagnosticBundle {
        DiagnosticBundle {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION,
            desktop: DiagnosticDesktop {
                app_version: "0.1.0".into(),
                platform: "linux".into(),
                generated_at_ms: 42,
            },
            config: DiagnosticConfig {
                executable_file: Some("pulsedagd".into()),
                rpc_endpoint: "http://127.0.0.1:8080".into(),
                data_directory: "<redacted-path>".into(),
                config_profile: "private".into(),
            },
            binary: Some(DiagnosticBinary {
                file_name: "pulsedagd".into(),
                size_bytes: 64,
                sha256: "a".repeat(64),
                executable: true,
            }),
            provenance: Some(DiagnosticProvenance {
                archive_name: "pulsedagd-v2.3.0-x86_64-unknown-linux-gnu.tar.gz".into(),
                archive_sha256: "b".repeat(64),
                release_tag: "v2.3.0".into(),
                source_commit: APPROVED_RELEASE_COMMIT.into(),
                target: "x86_64-unknown-linux-gnu".into(),
                embedded_binary_sha256: "a".repeat(64),
                binary_size_bytes: 64,
                linked_at_ms: 41,
            }),
            runtime: NodeRuntimeStatus {
                running: true,
                pid: Some(7),
                started_at_ms: Some(1),
                uptime_seconds: Some(41),
                last_exit_code: None,
                executable_path: Some("<redacted-path>".into()),
            },
            rpc_health: RpcHealth {
                reachable: true,
                status_code: Some(200),
                latency_ms: 3,
                message: "healthy".into(),
            },
            logs: vec![LogEntry {
                sequence: 1,
                timestamp_ms: 2,
                stream: "desktop".into(),
                message: "ready".into(),
            }],
            redactions_applied: vec!["node executable path".into()],
        }
    }

    #[test]
    fn diagnostic_schema_v2_contract_is_stable() {
        let value = serde_json::to_value(sample_bundle()).expect("serialize diagnostic bundle");
        let object = value.as_object().expect("diagnostic object");
        assert_eq!(object.len(), 9);
        for key in [
            "schemaVersion",
            "desktop",
            "config",
            "binary",
            "provenance",
            "runtime",
            "rpcHealth",
            "logs",
            "redactionsApplied",
        ] {
            assert!(object.contains_key(key), "missing diagnostic key {key}");
        }
        assert_eq!(value["schemaVersion"], DIAGNOSTIC_SCHEMA_VERSION);
        assert_eq!(value["config"]["dataDirectory"], "<redacted-path>");
        assert_eq!(value["runtime"]["executablePath"], "<redacted-path>");
        let encoded = serde_json::to_string(&value).expect("encode diagnostic value");
        assert!(!encoded.contains("schema_version"));
        assert!(!encoded.contains("rpc_health"));
    }

    #[test]
    fn diagnostic_log_window_is_bounded_ordered_and_redacted() {
        let config = NodeLaunchConfig {
            executable_path: "/home/operator/pulsedagd".into(),
            rpc_endpoint: "http://127.0.0.1:8080".into(),
            data_directory: "/home/operator/pulsedag-data".into(),
            config_profile: "private".into(),
        };
        let queue = (1..=600)
            .map(|sequence| LogEntry {
                sequence,
                timestamp_ms: sequence,
                stream: "stdout".into(),
                message: format!(
                    "binary=/home/operator/pulsedagd data=/home/operator/pulsedag-data item={sequence}"
                ),
            })
            .collect::<VecDeque<_>>();

        let logs = collect_diagnostic_logs(&queue, &config, Some(500));
        assert_eq!(logs.len(), 500);
        assert_eq!(logs.first().map(|entry| entry.sequence), Some(101));
        assert_eq!(logs.last().map(|entry| entry.sequence), Some(600));
        assert!(logs.iter().all(|entry| !entry.message.contains("/home/operator")));
        assert!(logs
            .iter()
            .all(|entry| entry.message.contains("<redacted-path>")));
        assert_eq!(normalize_log_window(None), DEFAULT_LOG_WINDOW);
        assert_eq!(normalize_log_window(Some(1)), MIN_LOG_WINDOW);
        assert_eq!(normalize_log_window(Some(9_999)), MAX_LOG_ENTRIES);
    }
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
            get_node_log_tail,
            clear_node_logs,
            export_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PulseDAG Desktop");
}
