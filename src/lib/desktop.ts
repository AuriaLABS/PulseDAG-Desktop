import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type {
  BinaryInfo,
  BinaryProvenance,
  BlockDetail,
  DesktopBridgeStatus,
  DiagnosticExportResult,
  LogBatch,
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
}

const stoppedStatus: NodeRuntimeStatus = {
  running: false,
  pid: null,
  startedAtMs: null,
  uptimeSeconds: null,
  lastExitCode: null,
  executablePath: null,
}

function selectedPath(value: string | string[] | null): string | null {
  return typeof value === 'string' ? value : null
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

export async function selectDataDirectory(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select the persistent PulseDAG data directory',
    directory: true,
    multiple: false,
  }))
}

export async function selectReleaseArchive(): Promise<string | null> {
  return selectedPath(await open({
    title: 'Select an official PulseDAG v2.3.0 release archive',
    directory: false,
    multiple: false,
    filters: [{ name: 'Release archives', extensions: ['zip', 'gz'] }],
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

export async function validateNodeBinary(path: string): Promise<BinaryInfo> {
  return invoke<BinaryInfo>('validate_node_binary', { path })
}

export async function verifyApprovedReleaseArchive(path: string): Promise<ReleaseVerification> {
  return invoke<ReleaseVerification>('verify_approved_release_archive', { path })
}

export async function bindBinaryToVerifiedArchive(
  archivePath: string,
  executablePath: string,
): Promise<BinaryProvenance> {
  return invoke<BinaryProvenance>('bind_binary_to_verified_archive', { archivePath, executablePath })
}

export async function getBinaryProvenance(): Promise<BinaryProvenance | null> {
  try {
    return await invoke<BinaryProvenance | null>('get_binary_provenance')
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

export async function getNodeLogs(after = 0, limit = 250): Promise<LogBatch> {
  try {
    return await invoke<LogBatch>('get_node_logs', { after, limit })
  } catch {
    return { entries: [], nextCursor: after }
  }
}

export async function clearNodeLogs(): Promise<void> {
  await invoke('clear_node_logs')
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
  })
}

export function loadNodePreferences(): NodePreferences {
  const raw = localStorage.getItem('pulsedag.desktop.node-preferences')
  if (!raw) return defaultPreferences

  try {
    const parsed = JSON.parse(raw) as Partial<NodePreferences> & { network?: string }
    const migratedProfile = parsed.configProfile
      ?? (parsed.network === 'private-testnet' ? 'private' : parsed.network === 'devnet' ? 'dev' : 'local')
    return { ...defaultPreferences, ...parsed, configProfile: migratedProfile } as NodePreferences
  } catch {
    return defaultPreferences
  }
}

export function saveNodePreferences(preferences: NodePreferences): void {
  localStorage.setItem('pulsedag.desktop.node-preferences', JSON.stringify(preferences))
}
