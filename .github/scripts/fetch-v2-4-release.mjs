import { createHash } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { appendFile, mkdir, rm, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'
import { Readable, Transform } from 'node:stream'
import { pipeline } from 'node:stream/promises'

const RELEASE_API = 'https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.4.0'
const RELEASE_TAG = 'v2.4.0'
const SOURCE_COMMIT = '876b48826a3875b729888edb88e2b0eea15bb717'
const SOURCE_BUILD_RUN_ID = '33070288236'
const MAX_ARCHIVE_BYTES = 512 * 1024 * 1024
const MAX_METADATA_BYTES = 2 * 1024 * 1024
const PACKAGED_RELEASE_NOTE = 'V2_4_0_KNOWN_LIMITATIONS.md'

const APPROVED_ASSETS = new Map([
  ['pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz', {
    kind: 'node', target: 'x86_64-unknown-linux-gnu', binaryName: 'pulsedagd',
    sha256: '27f777804f59beafc11ab9a5304818ebf1e9017dde171aa534721c5ed25301be',
  }],
  ['pulsedagd-v2.4.0-x86_64-pc-windows-msvc.zip', {
    kind: 'node', target: 'x86_64-pc-windows-msvc', binaryName: 'pulsedagd.exe',
    sha256: 'e282dac4fda1b7bc6ca9d3b0aef58aec2c64c5cd6ab8f4b0479d9af5f5a6baa6',
  }],
  ['pulsedagd-v2.4.0-x86_64-apple-darwin.tar.gz', {
    kind: 'node', target: 'x86_64-apple-darwin', binaryName: 'pulsedagd',
    sha256: 'fe7ec74bac2a8fd588969f98efae3dd379a95a56566ca71292ae821a624195d2',
  }],
  ['pulsedag-miner-v2.4.0-x86_64-unknown-linux-gnu.tar.gz', {
    kind: 'miner', target: 'x86_64-unknown-linux-gnu', binaryName: 'pulsedag-miner',
    sha256: '372fb7878183a161df433937e49422b69574f8e06e7092413c8ffbf70c3755e7',
  }],
  ['pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip', {
    kind: 'miner', target: 'x86_64-pc-windows-msvc', binaryName: 'pulsedag-miner.exe',
    sha256: '891c1cfae8c29a3f0f5e18c9e0363c2ca897de37c032927c45d36379c6174fea',
  }],
  ['pulsedag-miner-v2.4.0-x86_64-apple-darwin.tar.gz', {
    kind: 'miner', target: 'x86_64-apple-darwin', binaryName: 'pulsedag-miner',
    sha256: '5c0eaf24747dfedb4954e7cf4219a3644ff5b862e9c5389ff0baaa4c3dba4d4a',
  }],
])

const EXPECTED_RELEASE_ASSETS = new Set([
  ...[...APPROVED_ASSETS.keys()].flatMap((name) => [name, `${name}.sha256`, `${name}.json`]),
  'SHA256SUMS.txt',
  'INSTALL-VERIFY.md',
  'release-provenance.json',
])

function argument(name) {
  const index = process.argv.indexOf(name)
  if (index < 0 || !process.argv[index + 1]) throw new Error(`Missing required argument ${name}`)
  return process.argv[index + 1]
}

function apiHeaders() {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'PulseDAG-Desktop-v2.4-release-validation',
    'X-GitHub-Api-Version': '2022-11-28',
  }
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`
  return headers
}

function assertApprovedUrl(url, expectedName) {
  const parsed = new URL(url)
  if (
    parsed.protocol !== 'https:'
    || parsed.origin !== 'https://github.com'
    || parsed.pathname !== `/AuriaLABS/PulseDAG/releases/download/${RELEASE_TAG}/${expectedName}`
    || parsed.search
    || parsed.hash
  ) {
    throw new Error(`Release metadata returned an unexpected download URL for ${expectedName}`)
  }
}

function assertRedirectHost(url, label) {
  const parsed = new URL(url)
  if (
    parsed.protocol !== 'https:'
    || !(
      parsed.hostname === 'github.com'
      || parsed.hostname.endsWith('.githubusercontent.com')
      || parsed.hostname === 'release-assets.githubusercontent.com'
    )
  ) {
    throw new Error(`${label} redirected to an unapproved host: ${parsed.hostname}`)
  }
}

function releaseAsset(release, name) {
  const asset = release.assets.find((candidate) => candidate.name === name)
  if (!asset || asset.state !== 'uploaded') throw new Error(`${name} is not an uploaded v2.4.0 release asset`)
  assertApprovedUrl(asset.browser_download_url, name)
  return asset
}

async function downloadTextAsset(release, name) {
  const asset = releaseAsset(release, name)
  if (!Number.isSafeInteger(asset.size) || asset.size <= 0 || asset.size > MAX_METADATA_BYTES) {
    throw new Error(`${name} has an invalid metadata size`)
  }
  const response = await fetch(asset.browser_download_url, {
    headers: { 'User-Agent': 'PulseDAG-Desktop-v2.4-release-validation' },
    redirect: 'follow',
  })
  if (!response.ok) throw new Error(`${name} download failed with HTTP ${response.status}`)
  assertRedirectHost(response.url, name)
  const length = Number(response.headers.get('content-length') ?? 0)
  if (length > MAX_METADATA_BYTES) throw new Error(`${name} exceeded the metadata safety limit`)
  const text = await response.text()
  if (Buffer.byteLength(text, 'utf8') > MAX_METADATA_BYTES) throw new Error(`${name} exceeded the metadata safety limit`)
  return text
}

function validateManifest(manifest, archiveName, archiveSize) {
  const approved = APPROVED_ASSETS.get(archiveName)
  if (!approved) throw new Error(`Manifest names unapproved archive ${archiveName}`)
  const included = new Set(manifest.included_files ?? [])
  const expectedIncluded = new Set(['README.md', PACKAGED_RELEASE_NOTE])
  const includedMatches = included.size === expectedIncluded.size && [...included].every((value) => expectedIncluded.has(value))
  if (
    manifest.tag !== RELEASE_TAG
    || manifest.archive !== archiveName
    || manifest.archive_sha256?.toLowerCase() !== approved.sha256
    || manifest.archive_size_bytes !== archiveSize
    || manifest.target !== approved.target
    || manifest.binary !== approved.binaryName
    || !includedMatches
    || manifest.provenance?.repository !== 'AuriaLABS/PulseDAG'
    || manifest.provenance?.commit !== SOURCE_COMMIT
    || String(manifest.provenance?.github_run_id ?? '') !== SOURCE_BUILD_RUN_ID
    || String(manifest.provenance?.github_run_attempt ?? '') !== '1'
  ) {
    throw new Error(`Manifest for ${archiveName} does not match the frozen Task31 release identity`)
  }
}

function validateSidecar(text, archiveName) {
  const approved = APPROVED_ASSETS.get(archiveName)
  const fields = text.trim().split(/\s+/)
  if (fields.length !== 2 || fields[0].toLowerCase() !== approved.sha256 || basename(fields[1].replace(/^\*/, '')) !== archiveName) {
    throw new Error(`Checksum sidecar for ${archiveName} does not match the frozen digest`)
  }
}

function validateConsolidatedChecksums(text) {
  const seen = new Map()
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim()
    if (!line || line.startsWith('#')) continue
    const match = line.match(/^([0-9a-f]{64})\s+\*?(.+)$/i)
    if (!match) throw new Error('SHA256SUMS.txt contains an invalid line')
    const name = basename(match[2])
    const approved = APPROVED_ASSETS.get(name)
    if (!approved || match[1].toLowerCase() !== approved.sha256 || seen.has(name)) {
      throw new Error(`SHA256SUMS.txt contains an unexpected digest for ${name}`)
    }
    seen.set(name, match[1].toLowerCase())
  }
  if (seen.size !== APPROVED_ASSETS.size || [...APPROVED_ASSETS.keys()].some((name) => !seen.has(name))) {
    throw new Error('SHA256SUMS.txt does not cover exactly the six frozen v2.4.0 archives')
  }
}

function validateProvenanceSummary(summary, release) {
  if (
    summary.release_tag !== RELEASE_TAG
    || summary.source_sha !== SOURCE_COMMIT
    || summary.native_smoke_verified !== true
    || !Array.isArray(summary.artifacts)
    || summary.artifacts.length !== 6
  ) {
    throw new Error('release-provenance.json is incomplete, source-mismatched or not native-smoke verified')
  }
  const seen = new Set()
  for (const manifest of summary.artifacts) {
    const asset = releaseAsset(release, manifest.archive)
    validateManifest(manifest, manifest.archive, asset.size)
    if (seen.has(manifest.archive)) throw new Error(`Duplicate provenance manifest ${manifest.archive}`)
    seen.add(manifest.archive)
  }
  if (seen.size !== APPROVED_ASSETS.size || [...APPROVED_ASSETS.keys()].some((name) => !seen.has(name))) {
    throw new Error('release-provenance.json does not cover exactly the frozen archive allowlist')
  }
}

async function downloadAndHash(url, outputPath, expectedSize) {
  assertApprovedUrl(url, basename(outputPath))
  const response = await fetch(url, {
    headers: { 'User-Agent': 'PulseDAG-Desktop-v2.4-release-validation' },
    redirect: 'follow',
  })
  if (!response.ok || !response.body) throw new Error(`Release asset download failed with HTTP ${response.status}`)
  assertRedirectHost(response.url, 'Release archive')

  const declaredLength = Number(response.headers.get('content-length') ?? 0)
  if (declaredLength > MAX_ARCHIVE_BYTES) throw new Error('Release asset exceeds the 512 MiB safety limit')

  const hasher = createHash('sha256')
  let bytes = 0
  const meter = new Transform({
    transform(chunk, _encoding, callback) {
      bytes += chunk.length
      if (bytes > MAX_ARCHIVE_BYTES) {
        callback(new Error('Release asset exceeded the 512 MiB safety limit while downloading'))
        return
      }
      hasher.update(chunk)
      callback(null, chunk)
    },
  })

  try {
    await pipeline(Readable.fromWeb(response.body), meter, createWriteStream(outputPath, { flags: 'wx' }))
  } catch (error) {
    await rm(outputPath, { force: true })
    throw error
  }

  if (bytes !== expectedSize) throw new Error(`Downloaded ${bytes} bytes, but GitHub metadata declares ${expectedSize}`)
  return { bytes, sha256: hasher.digest('hex') }
}

async function appendEnvironment(values) {
  if (!process.env.GITHUB_ENV) return
  await appendFile(process.env.GITHUB_ENV, `${Object.entries(values).map(([key, value]) => `${key}=${value}`).join('\n')}\n`)
}

async function appendOutputs(values) {
  if (!process.env.GITHUB_OUTPUT) return
  await appendFile(process.env.GITHUB_OUTPUT, `${Object.entries(values).map(([key, value]) => `${key}=${value}`).join('\n')}\n`)
}

async function main() {
  const assetName = argument('--asset')
  const outputDirectory = resolve(argument('--output'))
  const approved = APPROVED_ASSETS.get(assetName)
  if (!approved) throw new Error(`Unsupported frozen v2.4.0 release asset ${assetName}`)

  await rm(outputDirectory, { recursive: true, force: true })
  await mkdir(outputDirectory, { recursive: true })

  const releaseResponse = await fetch(RELEASE_API, { headers: apiHeaders(), redirect: 'error' })
  if (!releaseResponse.ok) throw new Error(`Release metadata request failed with HTTP ${releaseResponse.status}`)
  const release = await releaseResponse.json()
  if (release.tag_name !== RELEASE_TAG || release.target_commitish !== SOURCE_COMMIT || release.draft || release.prerelease) {
    throw new Error('Published PulseDAG v2.4.0 release no longer matches the frozen Task31 identity')
  }
  const actualAssetNames = new Set(release.assets.map((asset) => asset.name))
  if (
    actualAssetNames.size !== EXPECTED_RELEASE_ASSETS.size
    || [...actualAssetNames].some((name) => !EXPECTED_RELEASE_ASSETS.has(name))
    || [...EXPECTED_RELEASE_ASSETS].some((name) => !actualAssetNames.has(name))
  ) {
    throw new Error('Published PulseDAG v2.4.0 release is not the exact frozen 21-file asset set')
  }
  if (release.assets.some((asset) => asset.state !== 'uploaded')) throw new Error('A frozen release asset is not uploaded')

  const asset = releaseAsset(release, assetName)
  if (!Number.isSafeInteger(asset.size) || asset.size <= 0 || asset.size > MAX_ARCHIVE_BYTES) {
    throw new Error('Release metadata contains an invalid archive size')
  }
  const githubDigest = asset.digest?.match(/^sha256:([0-9a-f]{64})$/i)?.[1]?.toLowerCase()
  if (githubDigest !== approved.sha256) throw new Error(`GitHub digest for ${assetName} no longer matches the frozen Task31 digest`)

  const sidecarText = await downloadTextAsset(release, `${assetName}.sha256`)
  validateSidecar(sidecarText, assetName)

  const manifestText = await downloadTextAsset(release, `${assetName}.json`)
  const manifest = JSON.parse(manifestText)
  validateManifest(manifest, assetName, asset.size)

  const sumsText = await downloadTextAsset(release, 'SHA256SUMS.txt')
  validateConsolidatedChecksums(sumsText)

  const provenanceText = await downloadTextAsset(release, 'release-provenance.json')
  const provenance = JSON.parse(provenanceText)
  validateProvenanceSummary(provenance, release)

  // INSTALL-VERIFY.md is a release-level guide. Each final archive contains
  // README.md plus V2_4_0_KNOWN_LIMITATIONS.md, exactly as Task31 built it.
  releaseAsset(release, 'INSTALL-VERIFY.md')

  const archivePath = join(outputDirectory, assetName)
  const downloaded = await downloadAndHash(asset.browser_download_url, archivePath, asset.size)
  if (downloaded.sha256 !== approved.sha256) {
    throw new Error(`Downloaded archive digest ${downloaded.sha256} does not match frozen digest ${approved.sha256}`)
  }

  const archiveRoot = assetName.endsWith('.tar.gz')
    ? assetName.slice(0, -'.tar.gz'.length)
    : assetName.slice(0, -'.zip'.length)
  const evidence = {
    schemaVersion: 2,
    repository: 'AuriaLABS/PulseDAG',
    releaseTag: RELEASE_TAG,
    sourceCommit: SOURCE_COMMIT,
    sourceBuildRunId: SOURCE_BUILD_RUN_ID,
    artifactKind: approved.kind,
    assetName,
    target: approved.target,
    sizeBytes: downloaded.bytes,
    sha256: downloaded.sha256,
    manifestVerified: true,
    consolidatedChecksumsVerified: true,
    nativeSmokeProvenanceVerified: true,
    verifiedAt: new Date().toISOString(),
  }
  const evidenceBase = approved.kind === 'miner' ? 'v24-miner-release-evidence.json' : 'v24-node-release-evidence.json'
  const evidencePath = join(outputDirectory, evidenceBase)
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, { flag: 'wx' })

  const environment = approved.kind === 'miner'
    ? {
        PULSEDAG_V24_MINER_ARCHIVE: archivePath,
        PULSEDAG_V24_MINER_ARCHIVE_SHA256: downloaded.sha256,
        PULSEDAG_V24_MINER_EVIDENCE: evidencePath,
        PULSEDAG_V24_MINER_ARCHIVE_ROOT: archiveRoot,
        PULSEDAG_V24_MINER_BINARY_NAME: approved.binaryName,
      }
    : {
        PULSEDAG_V24_NODE_ARCHIVE: archivePath,
        PULSEDAG_V24_NODE_ARCHIVE_SHA256: downloaded.sha256,
        PULSEDAG_V24_NODE_EVIDENCE: evidencePath,
        PULSEDAG_V24_NODE_ARCHIVE_ROOT: archiveRoot,
        PULSEDAG_V24_NODE_BINARY_NAME: approved.binaryName,
      }
  await appendEnvironment(environment)
  await appendOutputs({
    archive_path: archivePath,
    archive_sha256: downloaded.sha256,
    archive_root: archiveRoot,
    binary_name: approved.binaryName,
    evidence_path: evidencePath,
    target: approved.target,
    artifact_kind: approved.kind,
  })

  console.log(`Verified final ${assetName}`)
  console.log(`Source ${SOURCE_COMMIT}`)
  console.log(`SHA-256 ${downloaded.sha256}`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : error)
  process.exitCode = 1
})
