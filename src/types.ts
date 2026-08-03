export type AppSection = 'overview' | 'node' | 'network' | 'dag' | 'logs' | 'settings'

export type LogWindowSize = 250 | 500 | 1000 | 2000 | 5000

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
  logWindow: LogWindowSize
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

export type BinaryProvenance = {
  archiveName: string
  archiveSha256: string
  releaseTag: string
  sourceCommit: string
  target: string
  embeddedPath: string
  embeddedBinarySha256: string
  embeddedBinarySizeBytes: number
  selectedBinarySha256: string
  selectedBinarySizeBytes: number
  linkedAtMs: number
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

export type BlockOverview = {
  hash: string
  height: number
  blueScore: number
  timestamp: number
  parentHashes: string[]
  childHashes: string[]
  txCount: number
  txids: string[]
  isTip: boolean
  selectedTip: string | null
  confirmations: number
}

export type BlockTransaction = {
  txid: string
  fee: number
  inputs: number
  outputs: number
  context: string
  isConfirmed: boolean
  isMempool: boolean
}

export type BlockTransactions = {
  blockHash: string
  blockHeight: number
  count: number
  total: number
  limit: number
  offset: number
  hasMore: boolean
  context: string
  transactions: BlockTransaction[]
}

export type BlockDetail = {
  fetchedAtMs: number
  latencyMs: number
  overview: BlockOverview
  transactions: BlockTransactions
}

export type TransactionOutPoint = {
  txid: string
  index: number
}

export type TransactionOutput = {
  address: string
  amount: number
}

export type TransactionLookup = {
  txid: string
  status: string
  isMempool: boolean
  isConfirmed: boolean
  fee: number
  nonce: number
  blockHash: string | null
  blockHeight: number | null
  confirmations: number | null
  inputs: TransactionOutPoint[]
  outputs: TransactionOutput[]
}

export type TransactionDetail = {
  fetchedAtMs: number
  latencyMs: number
  transaction: TransactionLookup
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
