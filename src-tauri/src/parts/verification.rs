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
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("RPC endpoint must not contain credentials, query parameters or fragments.".into());
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err("Use the RPC origin only, for example http://127.0.0.1:8080.".into());
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| "RPC endpoint must include a port.".to_string())?;
    let (socket_addr, host_header) = match url.host() {
        Some(Host::Ipv4(address)) if address.is_loopback() => (
            SocketAddr::new(IpAddr::V4(address), port),
            format!("{address}:{port}"),
        ),
        Some(Host::Ipv6(address)) if address.is_loopback() => (
            SocketAddr::new(IpAddr::V6(address), port),
            format!("[{address}]:{port}"),
        ),
        Some(Host::Domain(domain)) if domain.eq_ignore_ascii_case("localhost") => {
            let address = std::net::Ipv4Addr::LOCALHOST;
            (
                SocketAddr::new(IpAddr::V4(address), port),
                format!("localhost:{port}"),
            )
        }
        _ => return Err("RPC must resolve to localhost, 127.0.0.1 or ::1.".into()),
    };

    Ok(ParsedRpcEndpoint {
        socket_addr,
        host_header,
    })
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

fn redaction_values(config: &NodeLaunchConfig) -> Vec<String> {
    let mut values = Vec::new();
    for raw in [&config.executable_path, &config.data_directory] {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            values.push(trimmed.to_string());
            if let Ok(canonical) = fs::canonicalize(trimmed) {
                values.push(canonical.to_string_lossy().into_owned());
            }
        }
    }
    for key in ["HOME", "USERPROFILE"] {
        if let Some(value) = env::var_os(key) {
            let value = value.to_string_lossy().into_owned();
            if !value.is_empty() {
                values.push(value);
            }
        }
    }
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values.dedup();
    values
}

fn redact_text(value: &str, redactions: &[String]) -> String {
    redactions.iter().fold(value.to_string(), |current, sensitive| {
        current.replace(sensitive, "<redacted-path>")
    })
}

fn safe_export_path(raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        return Err("Choose an output file for the diagnostic bundle.".into());
    }
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err("Diagnostic bundles must use the .json extension.".into());
    }
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| "Choose a diagnostic file inside an existing directory.".to_string())?;
    if !parent.is_dir() {
        return Err("The diagnostic output directory does not exist.".into());
    }
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err("Refusing to overwrite a symbolic link.".into());
        }
    }
    Ok(path)
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
async fn verify_approved_release_archive(path: String) -> Result<ReleaseVerification, String> {
    let archive_path = path.trim().to_string();
    let archive = tauri::async_runtime::spawn_blocking(move || {
        inspect_release_archive(Path::new(&archive_path))
    })
    .await
    .map_err(|error| format!("Release archive verification task failed: {error}"))??;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("PulseDAG-Desktop/0.1.0")
        .build()
        .map_err(|error| format!("Cannot initialize release verification client: {error}"))?;
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
        .ok_or_else(|| "The selected archive is not an asset of the approved v2.3.0 release.".to_string())?;

    let expected = if let Some(digest) = asset.digest.as_deref() {
        digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
            .map(str::to_string)
            .ok_or_else(|| "GitHub returned an unsupported release asset digest.".to_string())?
    } else {
        let checksum_name = format!("{}.sha256", archive.file_name);
        let checksum_asset = release
            .assets
            .iter()
            .find(|candidate| candidate.name == checksum_name)
            .ok_or_else(|| "The approved release does not include the archive checksum asset.".to_string())?;
        let approved_prefix = format!(
            "https://github.com/AuriaLABS/PulseDAG/releases/download/{APPROVED_RELEASE_TAG}/"
        );
        if !checksum_asset.browser_download_url.starts_with(&approved_prefix) {
            return Err("GitHub returned an unexpected checksum download URL.".into());
        }
        let checksum = client
            .get(&checksum_asset.browser_download_url)
            .send()
            .await
            .map_err(|error| format!("Cannot download the approved release checksum: {error}"))?
            .error_for_status()
            .map_err(|error| format!("GitHub rejected the checksum download: {error}"))?
            .text()
            .await
            .map_err(|error| format!("Cannot read the approved release checksum: {error}"))?;
        let mut fields = checksum.split_whitespace();
        let digest = fields
            .next()
            .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
            .ok_or_else(|| "The approved checksum file does not contain a valid SHA-256 digest.".to_string())?;
        let named_file = fields
            .next()
            .map(|value| value.trim_start_matches('*'))
            .ok_or_else(|| "The approved checksum file does not name its archive.".to_string())?;
        if named_file != archive.file_name {
            return Err("The approved checksum file names a different archive.".into());
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
            "Archive digest matches the asset published in the approved PulseDAG v2.3.0 GitHub release."
                .into()
        } else {
            "Archive digest does not match the approved GitHub release asset. Do not extract or run it."
                .into()
        },
    })
}

