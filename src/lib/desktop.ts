import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type {
  BinaryInfo,
  BinaryProvenance,
  BlockDetail,
  DesktopBridgeStatus,
  DiagnosticExportResult,
  LogBatch,
  LogWindowSize,
  MinerRuntimeStatus,
  NodeObservability,
  NodePreferences,
  NodeRuntimeStatus,
  ReleaseVerification,
  RpcHealth,
  TransactionDetail,
} from '../types'

export const defaultPreferences: NodePreferences = {
  executablePath: '',
  rpcEndpoint: 'http://127.0.0.1:8080',
  dataDirectory: '',
  configProfile: 'dev',
  launchOnStartup: false,
  logWindow: 2000,
  minerExecutablePath: '',
  minerAddress: '',
  minerThreads: Math.max(1, Math.min(4, navigator.hardwareConcurrency || 1)),
  minerMaxTries: 500000,
  minerSleepMs: 1000,
  minerRefreshBeforeExpiryMs: 1000,
  minerWorkerId: 'desktop-worker',
  minerHeartbeat: true,
}

const stoppedStatus: NodeRuntimeStatus = {
  running: false,
  pid: null,
  startedAtMs: null,
  uptimeSeconds: null,
  lastExitCode: null,
  executablePath: null,
}

const stoppedMinerStatus: MinerRuntimeStatus = {
  running: false,
  pid: null,
  startedAtMs: null,
  uptimeSeconds: null,
  lastExitCode: null,
  executablePath: null,
  telemetry: {
    lastEvent: null,
    backend: null,
    workers: null,
    attempts: 0,
    hashesPerSec: 0,
    templatesReceived: 0,
    templatesSkippedStale: 0,
    submitsTotal: 0,
    submitsAccepted: 0,
    submitsRejected: 0,
    lastRejectCode: null,
    lastTemplateHeight: null,
    lastAcceptedHeight: null,
    updatedAtMs: null,
  },
}

const supportedLogWindows: LogWindowSize[] = [250, 500, 1000, 2000, 5000]

function selectedPath(value: string | string[] | null): string | null {
  return typeof value === 'string' ? value : null
}

function normalizeLogWindow(value: unknown): LogWindowSize {
  return typeof value === 'number' && supportedLogWindows.includes(value as LogWindowSize)
    ? value as LogWindowSize
    : defaultPreferences.logWindow
}

function normalizeInteger(value: unknown, fallback: number, minimum: number, maximum: number): number {
  return typeof value === 'number' && Number.isInteger(value)
    ? Math.min(maximum, Math.max(minimum, value))
    : fallback
}

export async function getDesktopBridgeStatus(): Promise<DesktopBridgeStatus> {
  try {
    return await invoke<DesktopBridgeStatus>('get_desktop_status')
  } catch {
    return {
      appVersion: 'web-preview',
      platform: 'browser',
      nodeConfigured: false,
      nodeRunning: false,
      rpcReachable: false,
    }
  }
}

export async function selectNodeBinary(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select the pulsedagd executable',
    directory: false,
    multiple: false,
  }))
}

export async function selectMinerBinary(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select the pulsedag-miner executable',
    directory: false,
    multiple: false,
  }))
}

export async function selectDataDirectory(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select the persistent PulseDAG data directory',
    directory: true,
    multiple: false,
  }))
}

export async function selectReleaseArchive(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select an official PulseDAG v2.3.0 node release archive',
    directory: false,
    multiple: false,
    filters: [{ name: 'Release archives', extensions: ['zip', 'gz'] }],
  }))
}

export async function selectMinerReleaseArchive(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select an official PulseDAG v2.3.0 miner release archive',
    directory: false,
    multiple: false,
    filters: [{ name: 'Miner release archives', extensions: ['zip', 'gz'] }],
  }))
}

export async function selectDiagnosticOutput(): Promise<string | null> {
  return save({
    title: 'Export redacted PulseDAG diagnostics',
    defaultPath: 'pulsedag-diagnostics.json',
    filters: [{ name: 'JSON diagnostics', extensions: ['json'] }],
  })
}

export async function discoverNodeBinary(): Promise<BinaryInfo | null> {
  return invoke<BinaryInfo | null>('discover_node_binary')
}

export async function discoverMinerBinary(): Promise<BinaryInfo | null> {
  return invoke<BinaryInfo | null>('discover_miner_binary')
}

export async function validateNodeBinary(path: string): Promise<BinaryInfo> {
  return invoke<BinaryInfo>('validate_node_binary', { path })
}

export async function validateMinerBinary(path: string): Promise<BinaryInfo> {
  return invoke<BinaryInfo>('validate_miner_binary', { path })
}

export async function verifyApprovedReleaseArchive(path: string): Promise<ReleaseVerification> {
  return invoke<ReleaseVerification>('verify_approved_release_archive', { path })
}

export async function verifyApprovedMinerReleaseArchive(path: string): Promise<ReleaseVerification> {
  return invoke<ReleaseVerification>('verify_approved_miner_release_archive', { path })
}

export async function bindBinaryToVerifiedArchive(
  archivePath: string,
  executablePath: string,
): Promise<BinaryProvenance> {
  return invoke<BinaryProvenance>('bind_binary_to_verified_archive', { archivePath, executablePath })
}

export async function bindMinerBinaryToVerifiedArchive(
  archivePath: string,
  executablePath: string,
): Promise<BinaryProvenance> {
  return invoke<BinaryProvenance>('bind_miner_binary_to_verified_archive', { archivePath, executablePath })
}

export async function getBinaryProvenance(): Promise<BinaryProvenance | null> {
  try {
    return await invoke<BinaryProvenance | null>('get_binary_provenance')
  } catch {
    return null
  }
}

export async function getMinerBinaryProvenance(): Promise<BinaryProvenance | null> {
  try {
    return await invoke<BinaryProvenance | null>('get_miner_binary_provenance')
  } catch {
    return null
  }
}

export async function getNodeStatus(): Promise<NodeRuntimeStatus> {
  try {
    return await invoke<NodeRuntimeStatus>('get_node_status')
  } catch {
    return stoppedStatus
  }
}

export async function getMinerStatus(): Promise<MinerRuntimeStatus> {
  try {
    return await invoke<MinerRuntimeStatus>('get_miner_status')
  } catch {
    return stoppedMinerStatus
  }
}

export async function checkRpcHealth(endpoint: string): Promise<RpcHealth> {
  try {
    return await invoke<RpcHealth>('check_rpc_health', { endpoint })
  } catch (error) {
    return {
      reachable: false,
      statusCode: null,
      latencyMs: 0,
      message: error instanceof Error ? error.message : String(error),
    }
  }
}

export async function getNodeObservability(endpoint: string): Promise<NodeObservability> {
  return invoke<NodeObservability>('get_node_observability', { endpoint })
}

export async function getBlockDetail(endpoint: string, hash: string): Promise<BlockDetail> {
  return invoke<BlockDetail>('get_block_detail', { endpoint, hash })
}

export async function getTransactionDetail(endpoint: string, txid: string): Promise<TransactionDetail> {
  return invoke<TransactionDetail>('get_transaction_detail', { endpoint, txid })
}

export async function startNode(preferences: NodePreferences): Promise<NodeRuntimeStatus> {
  return invoke<NodeRuntimeStatus>('start_node', {
    config: {
      executablePath: preferences.executablePath,
      rpcEndpoint: preferences.rpcEndpoint,
      dataDirectory: preferences.dataDirectory,
      configProfile: preferences.configProfile,
    },
  })
}

export async function stopNode(): Promise<NodeRuntimeStatus> {
  return invoke<NodeRuntimeStatus>('stop_node')
}

export async function startMiner(preferences: NodePreferences): Promise<MinerRuntimeStatus> {
  const command = preferences.configProfile === 'private' ? 'start_verified_miner' : 'start_miner'
  return invoke<MinerRuntimeStatus>(command, {
    config: {
      executablePath: preferences.minerExecutablePath,
      nodeEndpoint: preferences.rpcEndpoint,
      minerAddress: preferences.minerAddress,
      configProfile: preferences.configProfile,
      threads: preferences.minerThreads,
      maxTries: preferences.minerMaxTries,
      sleepMs: preferences.minerSleepMs,
      refreshBeforeExpiryMs: preferences.minerRefreshBeforeExpiryMs,
      workerId: preferences.minerWorkerId,
      heartbeat: preferences.minerHeartbeat,
    },
  })
}

export async function stopMiner(): Promise<MinerRuntimeStatus> {
  return invoke<MinerRuntimeStatus>('stop_miner')
}

export async function getNodeLogs(after = 0, limit = 250): Promise<LogBatch> {
  try {
    return await invoke<LogBatch>('get_node_logs', { after, limit })
  } catch {
    return { entries: [], nextCursor: after }
  }
}

export async function getMinerLogs(after = 0, limit = 250): Promise<LogBatch> {
  try {
    return await invoke<LogBatch>('get_miner_logs', { after, limit })
  } catch {
    return { entries: [], nextCursor: after }
  }
}

export async function getNodeLogTail(limit: LogWindowSize): Promise<LogBatch> {
  try {
    return await invoke<LogBatch>('get_node_log_tail', { limit })
  } catch {
    return { entries: [], nextCursor: 0 }
  }
}

export async function clearNodeLogs(): Promise<void> {
  await invoke('clear_node_logs')
}

export async function clearMinerLogs(): Promise<void> {
  await invoke('clear_miner_logs')
}

export async function exportDiagnostics(
  outputPath: string,
  preferences: NodePreferences,
  rpcHealth: RpcHealth,
): Promise<DiagnosticExportResult> {
  return invoke<DiagnosticExportResult>('export_diagnostics', {
    outputPath,
    config: {
      executablePath: preferences.executablePath,
      rpcEndpoint: preferences.rpcEndpoint,
      dataDirectory: preferences.dataDirectory,
      configProfile: preferences.configProfile,
    },
    rpcHealth,
    logLimit: preferences.logWindow,
  })
}

export function loadNodePreferences(): NodePreferences {
  const raw = localStorage.getItem('pulsedag.desktop.node-preferences')
  if (!raw) return defaultPreferences

  try {
    const parsed = JSON.parse(raw) as Partial<NodePreferences> & { network?: string }
    const migratedProfile = parsed.configProfile
      ?? (parsed.network === 'private-testnet' ? 'private' : parsed.network === 'devnet' ? 'dev' : 'local')
    return {
      ...defaultPreferences,
      ...parsed,
      configProfile: migratedProfile,
      logWindow: normalizeLogWindow(parsed.logWindow),
      minerThreads: normalizeInteger(parsed.minerThreads, defaultPreferences.minerThreads, 1, 256),
      minerMaxTries: normalizeInteger(parsed.minerMaxTries, defaultPreferences.minerMaxTries, 1, 100000000),
      minerSleepMs: normalizeInteger(parsed.minerSleepMs, defaultPreferences.minerSleepMs, 100, 60000),
      minerRefreshBeforeExpiryMs: normalizeInteger(
        parsed.minerRefreshBeforeExpiryMs,
        defaultPreferences.minerRefreshBeforeExpiryMs,
        0,
        60000,
      ),
    } as NodePreferences
  } catch {
    return defaultPreferences
  }
}

export function saveNodePreferences(preferences: NodePreferences): void {
  localStorage.setItem('pulsedag.desktop.node-preferences', JSON.stringify(preferences))
}
