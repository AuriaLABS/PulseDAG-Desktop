import { createHash } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { appendFile, mkdir, rm, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'
import { Readable, Transform } from 'node:stream'
import { pipeline } from 'node:stream/promises'

const RELEASE_API = 'https://api.github.com/repos/AuriaLABS/PulseDAG/releases/tags/v2.3.0'
const RELEASE_TAG = 'v2.3.0'
const SOURCE_COMMIT = '7e43225f01ac05d15e5f1e3f1550d7850bf18cbc'
const MAX_ARCHIVE_BYTES = 512 * 1024 * 1024
const APPROVED_ASSETS = new Map([
  ['pulsedagd-v2.3.0-x86_64-unknown-linux-gnu.tar.gz', {
    kind: 'node',
    target: 'x86_64-unknown-linux-gnu',
    binaryName: 'pulsedagd',
  }],
  ['pulsedagd-v2.3.0-x86_64-pc-windows-msvc.zip', {
    kind: 'node',
    target: 'x86_64-pc-windows-msvc',
    binaryName: 'pulsedagd.exe',
  }],
  ['pulsedag-miner-v2.3.0-x86_64-unknown-linux-gnu.tar.gz', {
    kind: 'miner',
    target: 'x86_64-unknown-linux-gnu',
    binaryName: 'pulsedag-miner',
  }],
  ['pulsedag-miner-v2.3.0-x86_64-pc-windows-msvc.zip', {
    kind: 'miner',
    target: 'x86_64-pc-windows-msvc',
    binaryName: 'pulsedag-miner.exe',
  }],
])

function argument(name) {
  const index = process.argv.indexOf(name)
  if (index < 0 || !process.argv[index + 1]) {
    throw new Error(`Missing required argument ${name}`)
  }
  return process.argv[index + 1]
}

function apiHeaders() {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'PulseDAG-Desktop-native-packaging',
    'X-GitHub-Api-Version': '2022-11-28',
  }
  if (process.env.GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`
  }
  return headers
}

function assertApprovedUrl(url, expectedName) {
  const parsed = new URL(url)
  if (
    parsed.protocol !== 'https:'
    || parsed.origin !== 'https://github.com'
    || parsed.pathname !== `/AuriaLABS/PulseDAG/releases/download/${RELEASE_TAG}/${expectedName}`
  ) {
    throw new Error(`Release metadata returned an unexpected download URL for ${expectedName}`)
  }
}

async function responseText(response, label) {
  if (!response.ok) {
    throw new Error(`${label} failed with HTTP ${response.status}`)
  }
  return response.text()
}

async function expectedDigest(release, asset) {
  const digest = asset.digest?.match(/^sha256:([0-9a-f]{64})$/i)?.[1]
  if (digest) return digest.toLowerCase()

  const checksumName = `${asset.name}.sha256`
  const checksumAsset = release.assets.find((candidate) => candidate.name === checksumName)
  if (!checksumAsset) {
    throw new Error(`The approved release does not publish a digest or ${checksumName}`)
  }
  assertApprovedUrl(checksumAsset.browser_download_url, checksumName)
  const checksum = await responseText(
    await fetch(checksumAsset.browser_download_url, {
      headers: { 'User-Agent': 'PulseDAG-Desktop-native-packaging' },
      redirect: 'follow',
    }),
    'Checksum download',
  )
  const line = checksum
    .split(/\r?\n/)
    .map((value) => value.trim())
    .find((value) => value.endsWith(asset.name))
  const match = line?.match(/^([0-9a-f]{64})\s+\*?(.+)$/i)
  if (!match || basename(match[2]) !== asset.name) {
    throw new Error(`The checksum asset does not contain an exact digest for ${asset.name}`)
  }
  return match[1].toLowerCase()
}

async function downloadAndHash(url, outputPath, expectedSize) {
  assertApprovedUrl(url, basename(outputPath))
  const response = await fetch(url, {
    headers: { 'User-Agent': 'PulseDAG-Desktop-native-packaging' },
    redirect: 'follow',
  })
  if (!response.ok || !response.body) {
    throw new Error(`Release asset download failed with HTTP ${response.status}`)
  }
  const final = new URL(response.url)
  if (
    final.protocol !== 'https:'
    || !(
      final.hostname === 'github.com'
      || final.hostname.endsWith('.githubusercontent.com')
      || final.hostname === 'release-assets.githubusercontent.com'
    )
  ) {
    throw new Error(`Release asset redirected to an unapproved host: ${final.hostname}`)
  }

  const declaredLength = Number(response.headers.get('content-length') ?? 0)
  if (declaredLength > MAX_ARCHIVE_BYTES) {
    throw new Error('Release asset exceeds the 512 MiB safety limit')
  }

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
    await pipeline(
      Readable.fromWeb(response.body),
      meter,
      createWriteStream(outputPath, { flags: 'wx' }),
    )
  } catch (error) {
    await rm(outputPath, { force: true })
    throw error
  }

  if (expectedSize && bytes !== expectedSize) {
    throw new Error(`Downloaded ${bytes} bytes, but GitHub metadata declares ${expectedSize}`)
  }
  return { bytes, sha256: hasher.digest('hex') }
}

async function appendEnvironment(values) {
  if (!process.env.GITHUB_ENV) return
  const lines = Object.entries(values).map(([key, value]) => `${key}=${value}`).join('\n')
  await appendFile(process.env.GITHUB_ENV, `${lines}\n`)
}

async function appendOutputs(values) {
  if (!process.env.GITHUB_OUTPUT) return
  const lines = Object.entries(values).map(([key, value]) => `${key}=${value}`).join('\n')
  await appendFile(process.env.GITHUB_OUTPUT, `${lines}\n`)
}

async function main() {
  const assetName = argument('--asset')
  const outputDirectory = resolve(argument('--output'))
  const approved = APPROVED_ASSETS.get(assetName)
  if (!approved) {
    throw new Error(`Unsupported release asset ${assetName}`)
  }

  await rm(outputDirectory, { recursive: true, force: true })
  await mkdir(outputDirectory, { recursive: true })

  const releaseResponse = await fetch(RELEASE_API, {
    headers: apiHeaders(),
    redirect: 'error',
  })
  if (!releaseResponse.ok) {
    throw new Error(`Release metadata request failed with HTTP ${releaseResponse.status}`)
  }
  const release = await releaseResponse.json()
  if (
    release.tag_name !== RELEASE_TAG
    || release.target_commitish !== SOURCE_COMMIT
    || release.draft
    || release.prerelease
  ) {
    throw new Error('The published PulseDAG release no longer matches the approved identity')
  }

  const asset = release.assets.find((candidate) => candidate.name === assetName)
  if (!asset || asset.state !== 'uploaded') {
    throw new Error(`${assetName} is not an uploaded asset of the approved release`)
  }
  if (!Number.isSafeInteger(asset.size) || asset.size <= 0 || asset.size > MAX_ARCHIVE_BYTES) {
    throw new Error('Release metadata contains an invalid archive size')
  }
  assertApprovedUrl(asset.browser_download_url, assetName)

  const digest = await expectedDigest(release, asset)
  const archivePath = join(outputDirectory, assetName)
  const downloaded = await downloadAndHash(
    asset.browser_download_url,
    archivePath,
    asset.size,
  )
  if (downloaded.sha256 !== digest) {
    throw new Error(`Downloaded archive digest ${downloaded.sha256} does not match ${digest}`)
  }

  const archiveRoot = assetName.endsWith('.tar.gz')
    ? assetName.slice(0, -'.tar.gz'.length)
    : assetName.slice(0, -'.zip'.length)
  const evidence = {
    schemaVersion: 1,
    repository: 'AuriaLABS/PulseDAG',
    releaseTag: RELEASE_TAG,
    sourceCommit: SOURCE_COMMIT,
    artifactKind: approved.kind,
    assetName,
    target: approved.target,
    sizeBytes: downloaded.bytes,
    sha256: downloaded.sha256,
    verifiedAt: new Date().toISOString(),
  }
  const evidenceBase = approved.kind === 'miner' ? 'official-miner-release-evidence.json' : 'official-node-release-evidence.json'
  const checksumBase = approved.kind === 'miner' ? 'OFFICIAL_MINER_SHA256SUMS.txt' : 'OFFICIAL_NODE_SHA256SUMS.txt'
  const evidencePath = join(outputDirectory, evidenceBase)
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, { flag: 'wx' })
  await writeFile(
    join(outputDirectory, checksumBase),
    `${downloaded.sha256}  ${assetName}\n`,
    { flag: 'wx' },
  )

  const environment = approved.kind === 'miner'
    ? {
        PULSEDAG_MINER_PROVENANCE_ARCHIVE: archivePath,
        PULSEDAG_MINER_PROVENANCE_ARCHIVE_SHA256: downloaded.sha256,
        PULSEDAG_MINER_RELEASE_EVIDENCE: evidencePath,
        PULSEDAG_MINER_RELEASE_ARCHIVE_ROOT: archiveRoot,
        PULSEDAG_MINER_RELEASE_BINARY_NAME: approved.binaryName,
      }
    : {
        PULSEDAG_PROVENANCE_ARCHIVE: archivePath,
        PULSEDAG_PROVENANCE_ARCHIVE_SHA256: downloaded.sha256,
        PULSEDAG_RELEASE_EVIDENCE: evidencePath,
        PULSEDAG_RELEASE_ARCHIVE_ROOT: archiveRoot,
        PULSEDAG_RELEASE_BINARY_NAME: approved.binaryName,
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

  console.log(`Verified ${assetName}`)
  console.log(`SHA-256 ${downloaded.sha256}`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : error)
  process.exitCode = 1
})
