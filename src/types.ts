export type AppSection = 'overview' | 'node' | 'network' | 'dag' | 'logs' | 'settings'

export type DesktopBridgeStatus = {
  appVersion: string
  platform: string
  nodeConfigured: boolean
  nodeRunning: boolean
  rpcReachable: boolean
}

export type NodePreferences = {
  executablePath: string
  rpcEndpoint: string
  dataDirectory: string
  configProfile: 'dev' | 'local' | 'private'
  launchOnStartup: boolean
}

export type BinaryInfo = {
  path: string
  fileName: string
  sizeBytes: number
  sha256: string
  executable: boolean
}

export type NodeRuntimeStatus = {
  running: boolean
  pid: number | null
  startedAtMs: number | null
  uptimeSeconds: number | null
  lastExitCode: number | null
  executablePath: string | null
}

export type RpcHealth = {
  reachable: boolean
  statusCode: number | null
  latencyMs: number
  message: string
}

export type LogEntry = {
  sequence: number
  timestampMs: number
  stream: string
  message: string
}

export type LogBatch = {
  entries: LogEntry[]
  nextCursor: number
}
