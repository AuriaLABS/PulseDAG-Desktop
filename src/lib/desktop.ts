import { invoke } from '@tauri-apps/api/core'
import type {
  BinaryInfo,
  DesktopBridgeStatus,
  LogBatch,
  NodePreferences,
  NodeRuntimeStatus,
  RpcHealth,
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

export async function discoverNodeBinary(): Promise<BinaryInfo | null> {
  return invoke<BinaryInfo | null>('discover_node_binary')
}

export async function validateNodeBinary(path: string): Promise<BinaryInfo> {
  return invoke<BinaryInfo>('validate_node_binary', { path })
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
