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

export type ReleaseVerification = {
  archivePath: string
  archiveName: string
  sizeBytes: number
  sha256: string
  releaseTag: string
  sourceCommit: string
  assetDigest: string
  approved: boolean
  message: string
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

export type NodeStatusSnapshot = {
  rpcResponseDegraded: boolean
  rpcResponseStale: boolean
  rpcResponseDegradedReason: string | null
  networkId: string
  service: string
  version: string
  chainId: string
  bestHeight: number
  blockCount: number
  selectedTip: string | null
  selectedHeight: number | null
  consensusMode: string
  tipCount: number
  orphanCount: number
  mempoolSize: number
  snapshotHeight: number | null
  persistedBlockCount: number
  p2pMode: string | null
  peerCount: number
  syncState: string
  storageBackend: string
}

export type SyncStatusSnapshot = {
  rpcResponseDegraded: boolean
  rpcResponseStale: boolean
  consistencyOk: boolean
  consistencyIssueCount: number
  lagBlocks: number
  syncState: string
  networkSelectedHeightGap: number
  storageReplayGap: number
  liveSyncErrorActive: number
  p2pReadyForPrivateRehearsal: boolean
  readinessReasons: string[]
}

export type MempoolSnapshot = {
  transactionCount: number
  orphanTransactionCount: number
  orphanLimit: number
  spentOutpointsCount: number
  txids: string[]
}

export type PowHealthSnapshot = {
  status: string
  snapshotCount: number
  latestSuggestedDifficulty: number
  latestAvgBlockIntervalSecs: number
  alerts: string[]
}

export type RecentDagBlock = {
  hash: string
  height: number
  blueScore: number
  txCount: number
  timestamp: number
  parentCount: number
}

export type NodeObservability = {
  fetchedAtMs: number
  latencyMs: number
  status: NodeStatusSnapshot
  sync: SyncStatusSnapshot | null
  mempool: MempoolSnapshot | null
  pow: PowHealthSnapshot | null
  blocks: RecentDagBlock[]
  warnings: string[]
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

export type DiagnosticExportResult = {
  path: string
  bytesWritten: number
  logEntries: number
}
