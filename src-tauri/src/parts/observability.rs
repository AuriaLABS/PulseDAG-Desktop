use reqwest::{redirect::Policy, Client};
use serde::de::DeserializeOwned;

const MAX_RPC_RESPONSE_BYTES: usize = 1_048_576;
const OBSERVABILITY_STATUS_PATH: &str = "/api/v1/status";
const OBSERVABILITY_BLOCKS_PATH: &str = "/api/v1/blocks/recent?limit=20";
const OBSERVABILITY_SYNC_PATH: &str = "/api/v1/sync/status";
const OBSERVABILITY_MEMPOOL_PATH: &str = "/api/v1/mempool";
const OBSERVABILITY_POW_PATH: &str = "/api/v1/pow/health";

#[derive(Debug, Deserialize)]
struct ApiEnvelopeError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiEnvelopeError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct NodeStatusData {
    rpc_response_degraded: bool,
    rpc_response_stale: bool,
    rpc_response_degraded_reason: Option<String>,
    network_id: String,
    service: String,
    version: String,
    chain_id: String,
    best_height: u64,
    block_count: u64,
    selected_tip: Option<String>,
    selected_height: Option<u64>,
    consensus_mode: String,
    tip_count: u64,
    orphan_count: u64,
    mempool_size: u64,
    snapshot_height: Option<u64>,
    persisted_block_count: u64,
    p2p_mode: Option<String>,
    peer_count: u64,
    sync_state: String,
    storage_backend: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct SyncStatusData {
    rpc_response_degraded: bool,
    rpc_response_stale: bool,
    consistency_ok: bool,
    consistency_issue_count: u64,
    lag_blocks: u64,
    sync_state: String,
    network_selected_height_gap: u64,
    storage_replay_gap: u64,
    live_sync_error_active: u64,
    p2p_ready_for_private_rehearsal: bool,
    readiness_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct MempoolData {
    transaction_count: u64,
    orphan_transaction_count: u64,
    orphan_limit: u64,
    spent_outpoints_count: u64,
    txids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct PowHealthData {
    status: String,
    snapshot_count: u64,
    latest_suggested_difficulty: f64,
    latest_avg_block_interval_secs: f64,
    alerts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct RecentBlock {
    hash: String,
    height: u64,
    blue_score: u64,
    tx_count: u64,
    timestamp: u64,
    parent_count: u64,
}

#[derive(Debug, Deserialize)]
struct RecentBlocksData {
    blocks: Vec<RecentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeObservability {
    fetched_at_ms: u64,
    latency_ms: u128,
    status: NodeStatusData,
    sync: Option<SyncStatusData>,
    mempool: Option<MempoolData>,
    pow: Option<PowHealthData>,
    blocks: Vec<RecentBlock>,
    warnings: Vec<String>,
}

fn approved_observability_path(path: &str) -> bool {
    [
        OBSERVABILITY_STATUS_PATH,
        OBSERVABILITY_BLOCKS_PATH,
        OBSERVABILITY_SYNC_PATH,
        OBSERVABILITY_MEMPOOL_PATH,
        OBSERVABILITY_POW_PATH,
    ]
    .contains(&path)
}

fn observability_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_millis(1_500))
        .timeout(Duration::from_millis(3_000))
        .redirect(Policy::none())
        .user_agent("PulseDAG-Desktop/0.1 read-only-observability")
        .build()
        .map_err(|error| format!("Cannot create the local RPC client: {error}"))
}

async fn request_rpc_data<T: DeserializeOwned>(
    client: &Client,
    endpoint: &str,
    path: &str,
    label: &str,
) -> Result<T, String> {
    if !approved_observability_path(path) {
        return Err("The requested RPC path is outside the desktop read-only allowlist.".into());
    }

    let parsed = parse_local_rpc_endpoint(endpoint)?;
    let url = format!("http://{}{}", parsed.host_header, path);
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| format!("{label} request failed: {error}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("{label} response could not be read: {error}"))?;

    if bytes.len() > MAX_RPC_RESPONSE_BYTES {
        return Err(format!("{label} response exceeded the 1 MiB safety limit."));
    }
    if !status.is_success() {
        return Err(format!("{label} returned HTTP {}.", status.as_u16()));
    }

    let envelope: ApiEnvelope<T> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{label} returned invalid JSON: {error}"))?;
    if !envelope.ok {
        let detail = envelope
            .error
            .map(|error| format!("{} ({})", error.message, error.code))
            .unwrap_or_else(|| "unknown RPC error".into());
        return Err(format!("{label} was rejected by the node: {detail}"));
    }

    envelope
        .data
        .ok_or_else(|| format!("{label} returned an empty response."))
}

async fn optional_rpc_data<T: DeserializeOwned>(
    client: &Client,
    endpoint: &str,
    path: &str,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<T> {
    match request_rpc_data(client, endpoint, path, label).await {
        Ok(data) => Some(data),
        Err(error) => {
            warnings.push(error);
            None
        }
    }
}

fn push_unique_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warning.trim().is_empty() && !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

#[tauri::command]
async fn get_node_observability(endpoint: String) -> Result<NodeObservability, String> {
    parse_local_rpc_endpoint(&endpoint)?;
    let started = Instant::now();
    let client = observability_client()?;

    let status: NodeStatusData =
        request_rpc_data(&client, &endpoint, OBSERVABILITY_STATUS_PATH, "Node status").await?;
    let blocks: RecentBlocksData =
        request_rpc_data(&client, &endpoint, OBSERVABILITY_BLOCKS_PATH, "Recent blocks").await?;

    let mut warnings = Vec::new();
    let sync = optional_rpc_data(
        &client,
        &endpoint,
        OBSERVABILITY_SYNC_PATH,
        "Synchronization status",
        &mut warnings,
    )
    .await;
    let mempool = optional_rpc_data(
        &client,
        &endpoint,
        OBSERVABILITY_MEMPOOL_PATH,
        "Mempool status",
        &mut warnings,
    )
    .await;
    let pow = optional_rpc_data(
        &client,
        &endpoint,
        OBSERVABILITY_POW_PATH,
        "PoW health",
        &mut warnings,
    )
    .await;

    if let Some(reason) = status.rpc_response_degraded_reason.as_ref() {
        push_unique_warning(&mut warnings, reason.clone());
    }
    if status.rpc_response_degraded {
        push_unique_warning(&mut warnings, "Node status reports a degraded RPC snapshot.");
    }
    if status.rpc_response_stale {
        push_unique_warning(&mut warnings, "Node status reports stale RPC data.");
    }
    if let Some(sync_status) = sync.as_ref() {
        if !sync_status.consistency_ok {
            push_unique_warning(
                &mut warnings,
                format!(
                    "Synchronization consistency reports {} issue(s).",
                    sync_status.consistency_issue_count
                ),
            );
        }
        if sync_status.rpc_response_degraded {
            push_unique_warning(&mut warnings, "Synchronization status is degraded.");
        }
        if sync_status.rpc_response_stale {
            push_unique_warning(&mut warnings, "Synchronization status is stale.");
        }
    }
    if let Some(pow_health) = pow.as_ref() {
        for alert in &pow_health.alerts {
            push_unique_warning(&mut warnings, alert.clone());
        }
    }

    Ok(NodeObservability {
        fetched_at_ms: unix_time_ms(),
        latency_ms: started.elapsed().as_millis(),
        status,
        sync,
        mempool,
        pow,
        blocks: blocks.blocks,
        warnings,
    })
}

#[cfg(test)]
mod observability_tests {
    use super::*;

    #[test]
    fn observability_paths_are_exactly_allowlisted() {
        assert!(approved_observability_path(OBSERVABILITY_STATUS_PATH));
        assert!(approved_observability_path(OBSERVABILITY_BLOCKS_PATH));
        assert!(!approved_observability_path("/api/v1/admin/status"));
        assert!(!approved_observability_path("/api/v1/status?token=secret"));
    }
}
