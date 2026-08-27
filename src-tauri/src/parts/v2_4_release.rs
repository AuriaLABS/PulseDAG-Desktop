pub(crate) const V2_4_FINAL_RELEASE_TAG: &str = "v2.4.0";
pub(crate) const V2_4_FINAL_RELEASE_SOURCE_COMMIT: &str =
    "876b48826a3875b729888edb88e2b0eea15bb717";
pub(crate) const V2_4_FINAL_RELEASE_API: &str =
    "https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.4.0";
pub(crate) const V2_4_FINAL_RELEASE_REPOSITORY: &str = "AuriaLABS/PulseDAG";
pub(crate) const V2_4_FINAL_RELEASE_BUILD_RUN_ID: &str = "33070288236";

// The frozen v2.4.0 release workflow intentionally retained the historical
// install-document filename inside each archive. Trust the published bytes,
// not the provisional Desktop candidate assumption.
pub(crate) const V2_4_PACKAGED_INSTALL_GUIDE: &str = "INSTALL_BINARIES_V2_3_0.md";

const V2_4_FINAL_METADATA_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
struct V2_4FinalArchiveInspection {
    archive_path: String,
    archive_name: String,
    archive_sha256: String,
    archive_size_bytes: u64,
    binary_kind: V2_4CandidateBinaryKind,
    target: String,
    embedded_path: String,
    embedded_binary_sha256: String,
    embedded_binary_size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct V2_4GitHubReleaseAsset {
    name: String,
    digest: Option<String>,
    browser_download_url: String,
    size: u64,
    state: String,
}

#[derive(Debug, Deserialize)]
struct V2_4GitHubRelease {
    tag_name: String,
    target_commitish: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<V2_4GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct V2_4ManifestProvenance {
    repository: String,
    commit: String,
    github_run_id: String,
    github_run_attempt: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct V2_4ReleaseManifest {
    tag: String,
    archive: String,
    archive_sha256: String,
    archive_size_bytes: u64,
    target: String,
    binary: String,
    included_files: Vec<String>,
    provenance: V2_4ManifestProvenance,
}

#[derive(Debug, Deserialize)]
struct V2_4ReleaseProvenanceSummary {
    release_tag: String,
    artifacts: Vec<V2_4ReleaseManifest>,
    native_smoke_verified: bool,
}

fn v2_4_final_archive_digest(name: &str) -> Option<&'static str> {
    match name {
        "pulsedag-miner-v2.4.0-x86_64-apple-darwin.tar.gz" => {
            Some("5c0eaf24747dfedb4954e7cf4219a3644ff5b862e9c5389ff0baaa4c3dba4d4a")
        }
        "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz" => {
            Some("372fb7878183a161df433937e49422b69574f8e06e7092413c8ffbf70c3755e7")
        }
        "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip" => {
            Some("891c1cfae8c29a3f0f5e18c9e0363c2ca897de37c032927c45d36379c6174fea")
        }
        "pulsedagd-v2.4.0-x86_64-apple-darwin.tar.gz" => {
            Some("fe7ec74bac2a8fd588969f98efae3dd379a95a56566ca71292ae821a624195d2")
        }
        "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz" => {
            Some("27f777804f59beafc11ab9a5304818ebf1e9017dde171aa534721c5ed25301be")
        }
        "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip" => {
            Some("e282dac4fda1b7bc6ca9d3b0aef58aec2c64c5cd6ab8f4b0479d9af5f5a6baa6")
        }
        _ => None,
    }
}

fn v2_4_final_archive_names() -> [&'static str; 6] {
    [
        "pulsedag-miner-v2.4.0-x86_64-apple-darwin.tar.gz",
        "pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
        "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
        "pulsedagd-v2.4.0-x86_64-apple-darwin.tar.gz",
        "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
        "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip",
    ]
}

fn v2_4_expected_release_asset_names() -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    for archive in v2_4_final_archive_names() {
        names.insert(archive.to_string());
        names.insert(format!("{archive}.sha256"));
        names.insert(format!("{archive}.json"));
    }
    names.insert("SHA256SUMS.txt".into());
    names.insert("INSTALL-VERIFY.md".into());
    names.insert("release-provenance.json".into());
    names
}

fn v2_4_final_archive_layout(
    file_name: &str,
    binary_kind: V2_4CandidateBinaryKind,
) -> Result<ProvenanceArchiveLayout, String> {
    let (base_name, kind) = if let Some(base) = file_name.strip_suffix(".tar.gz") {
        (base.to_string(), ProvenanceArchiveKind::TarGz)
    } else if let Some(base) = file_name.strip_suffix(".zip") {
        (base.to_string(), ProvenanceArchiveKind::Zip)
    } else {
        return Err("A PulseDAG v2.4.0 release archive must be a .tar.gz or .zip file.".into());
    };

    let prefix = format!("{}-{V2_4_FINAL_RELEASE_TAG}-", binary_kind.archive_prefix());
    let target = base_name
        .strip_prefix(&prefix)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "The archive name does not identify the published PulseDAG v2.4.0 {} release.",
                binary_kind.public_name()
            )
        })?
        .to_string();

    if !matches!(
        target.as_str(),
        "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "x86_64-apple-darwin"
    ) {
        return Err("The v2.4.0 release archive target is not in the frozen release allowlist.".into());
    }

    let windows_target = target == "x86_64-pc-windows-msvc";
    match (kind, windows_target) {
        (ProvenanceArchiveKind::Zip, true) | (ProvenanceArchiveKind::TarGz, false) => {}
        _ => return Err("The v2.4.0 release archive format does not match its target.".into()),
    }

    let binary_name = match (binary_kind, windows_target) {
        (V2_4CandidateBinaryKind::Node, true) => "pulsedagd.exe",
        (V2_4CandidateBinaryKind::Node, false) => "pulsedagd",
        (V2_4CandidateBinaryKind::Miner, true) => "pulsedag-miner.exe",
        (V2_4CandidateBinaryKind::Miner, false) => "pulsedag-miner",
    }
    .to_string();
    let root = PathBuf::from(&base_name);
    let binary_path = root.join(&binary_name);
    let allowed_files = [
        binary_path.clone(),
        root.join("README.md"),
        root.join(V2_4_PACKAGED_INSTALL_GUIDE),
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

fn inspect_v2_4_final_archive_path(
    path: &Path,
    binary_kind: V2_4CandidateBinaryKind,
) -> Result<V2_4FinalArchiveInspection, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Cannot resolve the v2.4.0 release archive: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Cannot inspect the v2.4.0 release archive: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_PROVENANCE_ARCHIVE_BYTES {
        return Err("The v2.4.0 release archive is not a regular file within the safety limit.".into());
    }

    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The v2.4.0 release archive name is not valid UTF-8.".to_string())?;
    let layout = v2_4_final_archive_layout(file_name, binary_kind)?;
    let expected_digest = v2_4_final_archive_digest(file_name)
        .ok_or_else(|| "The archive is not one of the six frozen v2.4.0 release archives.".to_string())?;
    let host_target = supported_v2_4_candidate_target()?;
    if layout.target != host_target {
        return Err(format!(
            "The v2.4.0 release target {} does not match this desktop target {host_target}.",
            layout.target
        ));
    }

    let mut file = File::open(&canonical)
        .map_err(|error| format!("Cannot open the v2.4.0 release archive: {error}"))?;
    let (archive_sha256, archive_size_bytes) = hash_reader_limited(
        &mut file,
        MAX_PROVENANCE_ARCHIVE_BYTES,
        "the v2.4.0 release archive",
    )?;
    if archive_size_bytes != metadata.len() {
        return Err("The v2.4.0 release archive changed while it was being inspected.".into());
    }
    if !archive_sha256.eq_ignore_ascii_case(expected_digest) {
        return Err("The local archive digest does not match the frozen Task31 v2.4.0 digest.".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Cannot rewind the v2.4.0 release archive: {error}"))?;

    let evidence = match (binary_kind, layout.kind) {
        (V2_4CandidateBinaryKind::Node, ProvenanceArchiveKind::Zip) => {
            inspect_zip_binary(file, &layout, archive_sha256.clone())
        }
        (V2_4CandidateBinaryKind::Node, ProvenanceArchiveKind::TarGz) => {
            inspect_tar_binary(file, &layout, archive_sha256.clone())
        }
        (V2_4CandidateBinaryKind::Miner, ProvenanceArchiveKind::Zip) => {
            inspect_miner_zip_binary(file, &layout, archive_sha256.clone())
        }
        (V2_4CandidateBinaryKind::Miner, ProvenanceArchiveKind::TarGz) => {
            inspect_miner_tar_binary(file, &layout, archive_sha256.clone())
        }
    }?;

    Ok(V2_4FinalArchiveInspection {
        archive_path: canonical.to_string_lossy().into_owned(),
        archive_name: file_name.to_string(),
        archive_sha256,
        archive_size_bytes,
        binary_kind,
        target: evidence.target,
        embedded_path: evidence.embedded_path,
        embedded_binary_sha256: evidence.binary_sha256,
        embedded_binary_size_bytes: evidence.binary_size_bytes,
    })
}

fn validate_v2_4_release_download_url(url: &str, expected_name: &str) -> Result<(), String> {
    let parsed = Url::parse(url)
        .map_err(|error| format!("The v2.4.0 release returned an invalid download URL: {error}"))?;
    let expected_path = format!(
        "/AuriaLABS/PulseDAG/releases/download/{V2_4_FINAL_RELEASE_TAG}/{expected_name}"
    );
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || parsed.path() != expected_path
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(format!(
            "The v2.4.0 release returned an unexpected download URL for {expected_name}."
        ));
    }
    Ok(())
}

async fn download_v2_4_release_text(
    client: &reqwest::Client,
    asset: &V2_4GitHubReleaseAsset,
) -> Result<String, String> {
    if asset.size == 0 || asset.size > V2_4_FINAL_METADATA_BYTES {
        return Err(format!(
            "The published metadata asset {} is outside the safety bound.",
            asset.name
        ));
    }
    validate_v2_4_release_download_url(&asset.browser_download_url, &asset.name)?;
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|error| format!("Cannot download {}: {error}", asset.name))?
        .error_for_status()
        .map_err(|error| format!("GitHub rejected {}: {error}", asset.name))?;
    let final_url = response.url();
    let final_host = final_url.host_str().unwrap_or_default();
    if final_url.scheme() != "https"
        || !(final_host == "github.com"
            || final_host.ends_with(".githubusercontent.com")
            || final_host == "release-assets.githubusercontent.com")
    {
        return Err(format!(
            "The published metadata asset {} redirected to an unapproved host.",
            asset.name
        ));
    }
    if response.content_length().is_some_and(|length| length > V2_4_FINAL_METADATA_BYTES) {
        return Err(format!("The published metadata asset {} is too large.", asset.name));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Cannot read {}: {error}", asset.name))?;
    if bytes.len() as u64 > V2_4_FINAL_METADATA_BYTES {
        return Err(format!("The published metadata asset {} exceeded the safety bound.", asset.name));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| format!("The published metadata asset {} is not UTF-8.", asset.name))
}

fn release_asset<'a>(
    release: &'a V2_4GitHubRelease,
    name: &str,
) -> Result<&'a V2_4GitHubReleaseAsset, String> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .ok_or_else(|| format!("The v2.4.0 release is missing required asset {name}."))
}

fn validate_v2_4_manifest(
    manifest: &V2_4ReleaseManifest,
    expected_archive: &str,
    expected_size: u64,
) -> Result<(), String> {
    let expected_digest = v2_4_final_archive_digest(expected_archive)
        .ok_or_else(|| "The manifest names an archive outside the frozen v2.4.0 allowlist.".to_string())?;
    let kind = if expected_archive.starts_with("pulsedagd-") {
        V2_4CandidateBinaryKind::Node
    } else if expected_archive.starts_with("pulsedag-miner-") {
        V2_4CandidateBinaryKind::Miner
    } else {
        return Err("The manifest names an unsupported v2.4.0 binary family.".into());
    };
    let layout = v2_4_final_archive_layout(expected_archive, kind)?;
    let included = manifest
        .included_files
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let expected_included = ["README.md".to_string(), V2_4_PACKAGED_INSTALL_GUIDE.to_string()]
        .into_iter()
        .collect::<std::collections::HashSet<_>>();

    if manifest.tag != V2_4_FINAL_RELEASE_TAG
        || manifest.archive != expected_archive
        || !manifest.archive_sha256.eq_ignore_ascii_case(expected_digest)
        || manifest.archive_size_bytes != expected_size
        || manifest.target != layout.target
        || manifest.binary != layout.binary_name
        || included != expected_included
        || manifest.provenance.repository != V2_4_FINAL_RELEASE_REPOSITORY
        || manifest.provenance.commit != V2_4_FINAL_RELEASE_SOURCE_COMMIT
        || manifest.provenance.github_run_id != V2_4_FINAL_RELEASE_BUILD_RUN_ID
        || manifest.provenance.github_run_attempt != "1"
    {
        return Err(format!(
            "The published manifest for {expected_archive} does not match the frozen Task31 release identity."
        ));
    }
    Ok(())
}

fn validate_v2_4_checksum_text(
    text: &str,
    archive_name: &str,
    expected_digest: &str,
) -> Result<(), String> {
    let mut fields = text.split_whitespace();
    let digest = fields
        .next()
        .ok_or_else(|| "The checksum sidecar is empty.".to_string())?;
    let named = fields
        .next()
        .map(|value| value.trim_start_matches('*'))
        .ok_or_else(|| "The checksum sidecar does not name its archive.".to_string())?;
    if fields.next().is_some()
        || !digest.eq_ignore_ascii_case(expected_digest)
        || named != archive_name
    {
        return Err("The checksum sidecar does not exactly match the frozen archive identity.".into());
    }
    Ok(())
}

fn validate_v2_4_consolidated_checksums(text: &str) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let digest = fields
            .next()
            .ok_or_else(|| "SHA256SUMS.txt contains an invalid line.".to_string())?;
        let archive = fields
            .next()
            .map(|value| value.trim_start_matches('*'))
            .ok_or_else(|| "SHA256SUMS.txt contains an unnamed digest.".to_string())?;
        if fields.next().is_some() {
            return Err("SHA256SUMS.txt contains unexpected fields.".into());
        }
        let expected = v2_4_final_archive_digest(archive)
            .ok_or_else(|| "SHA256SUMS.txt names an archive outside the frozen allowlist.".to_string())?;
        if !digest.eq_ignore_ascii_case(expected) || !seen.insert(archive.to_string()) {
            return Err("SHA256SUMS.txt does not exactly match the frozen v2.4.0 digests.".into());
        }
    }
    let expected = v2_4_final_archive_names()
        .into_iter()
        .map(str::to_string)
        .collect::<std::collections::HashSet<_>>();
    if seen != expected {
        return Err("SHA256SUMS.txt does not cover exactly the six frozen release archives.".into());
    }
    Ok(())
}

fn validate_v2_4_provenance_summary(
    summary: &V2_4ReleaseProvenanceSummary,
    release: &V2_4GitHubRelease,
) -> Result<(), String> {
    if summary.release_tag != V2_4_FINAL_RELEASE_TAG
        || !summary.native_smoke_verified
        || summary.artifacts.len() != 6
    {
        return Err("The consolidated v2.4.0 provenance summary is incomplete or not smoke-verified.".into());
    }
    let mut seen = std::collections::HashSet::new();
    for manifest in &summary.artifacts {
        let asset = release_asset(release, &manifest.archive)?;
        validate_v2_4_manifest(manifest, &manifest.archive, asset.size)?;
        if !seen.insert(manifest.archive.clone()) {
            return Err("The consolidated v2.4.0 provenance summary contains duplicate archives.".into());
        }
    }
    let expected = v2_4_final_archive_names()
        .into_iter()
        .map(str::to_string)
        .collect::<std::collections::HashSet<_>>();
    if seen != expected {
        return Err("The consolidated v2.4.0 provenance summary does not cover all frozen archives.".into());
    }
    Ok(())
}

pub(crate) fn is_final_v2_4_release_provenance(proof: &TrustedBinaryProvenance) -> bool {
    if proof.release_tag != V2_4_FINAL_RELEASE_TAG
        || proof.source_commit != V2_4_FINAL_RELEASE_SOURCE_COMMIT
    {
        return false;
    }
    let Some(expected_digest) = v2_4_final_archive_digest(&proof.archive_name) else {
        return false;
    };
    if !proof.archive_sha256.eq_ignore_ascii_case(expected_digest) {
        return false;
    }
    let kind = if proof.archive_name.starts_with("pulsedagd-") {
        V2_4CandidateBinaryKind::Node
    } else if proof.archive_name.starts_with("pulsedag-miner-") {
        V2_4CandidateBinaryKind::Miner
    } else {
        return false;
    };
    let Ok(layout) = v2_4_final_archive_layout(&proof.archive_name, kind) else {
        return false;
    };
    proof.target == layout.target
        && proof.embedded_path == layout.binary_path.display().to_string()
}

#[tauri::command]
async fn verify_v2_4_release_archive(
    path: String,
    binary_kind: String,
) -> Result<ReleaseVerification, String> {
    let kind = V2_4CandidateBinaryKind::parse(&binary_kind)?;
    let task_path = path.trim().to_string();
    let inspected = tauri::async_runtime::spawn_blocking(move || {
        inspect_v2_4_final_archive_path(Path::new(&task_path), kind)
    })
    .await
    .map_err(|error| format!("v2.4.0 archive verification task failed: {error}"))??;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(4))
        .user_agent("PulseDAG-Desktop/0.1.0")
        .build()
        .map_err(|error| format!("Cannot initialize v2.4.0 release verification: {error}"))?;
    let release = client
        .get(V2_4_FINAL_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| format!("Cannot query the published PulseDAG v2.4.0 release: {error}"))?
        .error_for_status()
        .map_err(|error| format!("GitHub rejected the v2.4.0 release query: {error}"))?
        .json::<V2_4GitHubRelease>()
        .await
        .map_err(|error| format!("Cannot decode the published v2.4.0 release metadata: {error}"))?;

    if release.tag_name != V2_4_FINAL_RELEASE_TAG
        || release.target_commitish != V2_4_FINAL_RELEASE_SOURCE_COMMIT
        || release.draft
        || release.prerelease
    {
        return Err("The published v2.4.0 release no longer matches the Task31 frozen identity.".into());
    }
    let actual_assets = release
        .assets
        .iter()
        .map(|asset| asset.name.clone())
        .collect::<std::collections::HashSet<_>>();
    if actual_assets != v2_4_expected_release_asset_names() {
        return Err("The published v2.4.0 release asset set is not the frozen 21-file allowlist.".into());
    }
    if release.assets.iter().any(|asset| asset.state != "uploaded") {
        return Err("At least one frozen v2.4.0 release asset is not in uploaded state.".into());
    }

    let archive_asset = release_asset(&release, &inspected.archive_name)?;
    validate_v2_4_release_download_url(
        &archive_asset.browser_download_url,
        &inspected.archive_name,
    )?;
    if archive_asset.size != inspected.archive_size_bytes {
        return Err("The local v2.4.0 archive size does not match published GitHub metadata.".into());
    }
    let expected_digest = v2_4_final_archive_digest(&inspected.archive_name)
        .expect("final archive was already allowlisted");
    let published_digest = archive_asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
        .ok_or_else(|| "GitHub did not publish a valid SHA-256 digest for the v2.4.0 archive.".to_string())?;
    if !published_digest.eq_ignore_ascii_case(expected_digest)
        || !inspected.archive_sha256.eq_ignore_ascii_case(expected_digest)
    {
        return Err("The v2.4.0 archive digest does not match both Task31 and GitHub release evidence.".into());
    }

    let sidecar_name = format!("{}.sha256", inspected.archive_name);
    let sidecar = release_asset(&release, &sidecar_name)?;
    let sidecar_text = download_v2_4_release_text(&client, sidecar).await?;
    validate_v2_4_checksum_text(&sidecar_text, &inspected.archive_name, expected_digest)?;

    let manifest_name = format!("{}.json", inspected.archive_name);
    let manifest_asset = release_asset(&release, &manifest_name)?;
    let manifest_text = download_v2_4_release_text(&client, manifest_asset).await?;
    let manifest = serde_json::from_str::<V2_4ReleaseManifest>(&manifest_text)
        .map_err(|error| format!("Cannot decode {manifest_name}: {error}"))?;
    validate_v2_4_manifest(&manifest, &inspected.archive_name, inspected.archive_size_bytes)?;

    let sums_asset = release_asset(&release, "SHA256SUMS.txt")?;
    let sums_text = download_v2_4_release_text(&client, sums_asset).await?;
    validate_v2_4_consolidated_checksums(&sums_text)?;

    let provenance_asset = release_asset(&release, "release-provenance.json")?;
    let provenance_text = download_v2_4_release_text(&client, provenance_asset).await?;
    let provenance = serde_json::from_str::<V2_4ReleaseProvenanceSummary>(&provenance_text)
        .map_err(|error| format!("Cannot decode release-provenance.json: {error}"))?;
    validate_v2_4_provenance_summary(&provenance, &release)?;

    Ok(ReleaseVerification {
        archive_path: inspected.archive_path,
        archive_name: inspected.archive_name,
        size_bytes: inspected.archive_size_bytes,
        sha256: inspected.archive_sha256,
        release_tag: V2_4_FINAL_RELEASE_TAG.into(),
        source_commit: V2_4_FINAL_RELEASE_SOURCE_COMMIT.into(),
        asset_digest: format!("sha256:{expected_digest}"),
        approved: true,
        message: format!(
            "PulseDAG v2.4.0 {} archive matches the Task31 frozen SHA, GitHub digest, per-asset checksum/manifest, consolidated checksums and native-smoke provenance summary.",
            inspected.binary_kind.public_name()
        ),
    })
}

#[cfg(test)]
mod v2_4_final_release_tests {
    use super::*;

    fn proof(archive_name: &str, archive_sha256: &str, source_commit: &str) -> TrustedBinaryProvenance {
        let kind = if archive_name.starts_with("pulsedagd-") {
            V2_4CandidateBinaryKind::Node
        } else {
            V2_4CandidateBinaryKind::Miner
        };
        let layout = v2_4_final_archive_layout(archive_name, kind).unwrap();
        TrustedBinaryProvenance {
            executable_path: "/tmp/release-binary".into(),
            binary_sha256: "a".repeat(64),
            binary_size_bytes: 4,
            archive_name: archive_name.into(),
            archive_sha256: archive_sha256.into(),
            release_tag: V2_4_FINAL_RELEASE_TAG.into(),
            source_commit: source_commit.into(),
            target: layout.target,
            embedded_path: layout.binary_path.display().to_string(),
            linked_at_ms: 1,
        }
    }

    #[test]
    fn v2_4_final_release_asset_allowlist_is_exact() {
        assert_eq!(v2_4_final_archive_names().len(), 6);
        assert_eq!(v2_4_expected_release_asset_names().len(), 21);
        for archive in v2_4_final_archive_names() {
            let digest = v2_4_final_archive_digest(archive).expect("frozen digest");
            assert_eq!(digest.len(), 64);
            assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn v2_4_final_layout_matches_frozen_published_archive_contents() {
        let node = v2_4_final_archive_layout(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip",
            V2_4CandidateBinaryKind::Node,
        )
        .unwrap();
        assert!(node.allowed_files.contains(&PathBuf::from(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc/INSTALL_BINARIES_V2_3_0.md"
        )));
        assert!(!node.allowed_files.contains(&PathBuf::from(
            "pulsedagd-v2.4.0-x86_64-pc-windows-msvc/INSTALL_BINARIES_V2_4_0.md"
        )));
        assert_eq!(node.allowed_files.len(), 3);
    }

    #[test]
    fn v2_4_final_provenance_requires_source_archive_and_digest() {
        let archive = "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz";
        let digest = v2_4_final_archive_digest(archive).unwrap();
        assert!(is_final_v2_4_release_provenance(&proof(
            archive,
            digest,
            V2_4_FINAL_RELEASE_SOURCE_COMMIT
        )));
        assert!(!is_final_v2_4_release_provenance(&proof(
            archive,
            digest,
            "265bf83e8f58e1c1cedc3a6467f334d60d9ef283"
        )));
        assert!(!is_final_v2_4_release_provenance(&proof(
            archive,
            &"0".repeat(64),
            V2_4_FINAL_RELEASE_SOURCE_COMMIT
        )));
    }

    #[test]
    fn v2_4_manifest_is_source_and_native_build_bound() {
        let archive = "pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip";
        let digest = v2_4_final_archive_digest(archive).unwrap();
        let manifest = V2_4ReleaseManifest {
            tag: V2_4_FINAL_RELEASE_TAG.into(),
            archive: archive.into(),
            archive_sha256: digest.into(),
            archive_size_bytes: 123,
            target: "x86_64-pc-windows-msvc".into(),
            binary: "pulsedagd.exe".into(),
            included_files: vec!["README.md".into(), V2_4_PACKAGED_INSTALL_GUIDE.into()],
            provenance: V2_4ManifestProvenance {
                repository: V2_4_FINAL_RELEASE_REPOSITORY.into(),
                commit: V2_4_FINAL_RELEASE_SOURCE_COMMIT.into(),
                github_run_id: V2_4_FINAL_RELEASE_BUILD_RUN_ID.into(),
                github_run_attempt: "1".into(),
            },
        };
        validate_v2_4_manifest(&manifest, archive, 123).expect("frozen manifest");
        let mut stale = manifest.clone();
        stale.provenance.commit = "265bf83e8f58e1c1cedc3a6467f334d60d9ef283".into();
        assert!(validate_v2_4_manifest(&stale, archive, 123).is_err());
    }
}