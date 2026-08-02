const BLOCK_TRANSACTION_LIMIT: usize = 100;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BlockOverviewData {
    hash: String,
    height: u64,
    blue_score: u64,
    timestamp: u64,
    parent_hashes: Vec<String>,
    child_hashes: Vec<String>,
    tx_count: u64,
    txids: Vec<String>,
    is_tip: bool,
    selected_tip: Option<String>,
    confirmations: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BlockTransactionItem {
    txid: String,
    fee: u64,
    inputs: u64,
    outputs: u64,
    context: String,
    is_confirmed: bool,
    is_mempool: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BlockTransactionsData {
    block_hash: String,
    block_height: u64,
    count: u64,
    total: u64,
    limit: u64,
    offset: u64,
    has_more: bool,
    context: String,
    transactions: Vec<BlockTransactionItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TxLookupOutPoint {
    txid: String,
    index: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TxLookupOutput {
    address: String,
    amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TransactionLookupData {
    txid: String,
    status: String,
    is_mempool: bool,
    is_confirmed: bool,
    fee: u64,
    nonce: u64,
    block_hash: Option<String>,
    block_height: Option<u64>,
    confirmations: Option<u64>,
    inputs: Vec<TxLookupOutPoint>,
    outputs: Vec<TxLookupOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockDetail {
    fetched_at_ms: u64,
    latency_ms: u128,
    overview: BlockOverviewData,
    transactions: BlockTransactionsData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDetail {
    fetched_at_ms: u64,
    latency_ms: u128,
    transaction: TransactionLookupData,
}

fn validated_entity_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{label} must be an exact 64-character hexadecimal identifier returned by the node."
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn approved_entity_path(path: &str) -> bool {
    if let Some(hash) = path
        .strip_prefix("/api/v1/blocks/")
        .and_then(|value| value.strip_suffix("/overview"))
    {
        return validated_entity_id(hash, "Block hash").is_ok();
    }
    if let Some(hash) = path
        .strip_prefix("/api/v1/blocks/")
        .and_then(|value| value.strip_suffix("/transactions?limit=100&offset=0"))
    {
        return validated_entity_id(hash, "Block hash").is_ok();
    }
    if let Some(txid) = path
        .strip_prefix("/api/v1/txs/")
        .and_then(|value| value.strip_suffix("/lookup"))
    {
        return validated_entity_id(txid, "Transaction id").is_ok();
    }
    false
}

async fn request_entity_data<T: serde::de::DeserializeOwned>(
    client: &Client,
    endpoint: &str,
    path: &str,
    label: &str,
) -> Result<T, String> {
    if !approved_entity_path(path) {
        return Err("The requested entity path is outside the bounded desktop allowlist.".into());
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

#[tauri::command]
async fn get_block_detail(endpoint: String, hash: String) -> Result<BlockDetail, String> {
    parse_local_rpc_endpoint(&endpoint)?;
    let hash = validated_entity_id(&hash, "Block hash")?;
    let started = Instant::now();
    let client = observability_client()?;
    let overview_path = format!("/api/v1/blocks/{hash}/overview");
    let transactions_path = format!(
        "/api/v1/blocks/{hash}/transactions?limit={BLOCK_TRANSACTION_LIMIT}&offset=0"
    );

    let overview: BlockOverviewData =
        request_entity_data(&client, &endpoint, &overview_path, "Block overview").await?;
    let transactions: BlockTransactionsData = request_entity_data(
        &client,
        &endpoint,
        &transactions_path,
        "Block transactions",
    )
    .await?;

    Ok(BlockDetail {
        fetched_at_ms: unix_time_ms(),
        latency_ms: started.elapsed().as_millis(),
        overview,
        transactions,
    })
}

#[tauri::command]
async fn get_transaction_detail(
    endpoint: String,
    txid: String,
) -> Result<TransactionDetail, String> {
    parse_local_rpc_endpoint(&endpoint)?;
    let txid = validated_entity_id(&txid, "Transaction id")?;
    let started = Instant::now();
    let client = observability_client()?;
    let path = format!("/api/v1/txs/{txid}/lookup");
    let transaction: TransactionLookupData =
        request_entity_data(&client, &endpoint, &path, "Transaction lookup").await?;

    Ok(TransactionDetail {
        fetched_at_ms: unix_time_ms(),
        latency_ms: started.elapsed().as_millis(),
        transaction,
    })
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    #[test]
    fn entity_read_only_routes_are_bounded() {
        let hash = "a".repeat(64);
        assert!(approved_entity_path(&format!(
            "/api/v1/blocks/{hash}/overview"
        )));
        assert!(approved_entity_path(&format!(
            "/api/v1/blocks/{hash}/transactions?limit=100&offset=0"
        )));
        assert!(approved_entity_path(&format!(
            "/api/v1/txs/{hash}/lookup"
        )));
        assert!(!approved_entity_path(&format!(
            "/api/v1/blocks/{hash}/transactions?limit=1000&offset=0"
        )));
        assert!(!approved_entity_path("/api/v1/blocks/../admin/overview"));
        assert!(!approved_entity_path(&format!(
            "/api/v1/txs/{hash}/lookup?token=secret"
        )));
    }

    #[test]
    fn entity_identifiers_reject_path_injection() {
        assert!(validated_entity_id(&"b".repeat(64), "Hash").is_ok());
        assert!(validated_entity_id(&"b".repeat(63), "Hash").is_err());
        assert!(validated_entity_id(&format!("{}?", "b".repeat(63)), "Hash").is_err());
        assert!(validated_entity_id(&format!("{}/", "b".repeat(63)), "Hash").is_err());
    }
}
