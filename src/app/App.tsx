import { useCallback, useEffect, useRef, useState } from 'react'
import { Sidebar } from '../components/Sidebar'
import {
  bindBinaryToVerifiedArchive,
  checkRpcHealth,
  clearNodeLogs,
  discoverNodeBinary,
  exportDiagnostics,
  getBinaryProvenance,
  getDesktopBridgeStatus,
  getNodeLogs,
  getNodeObservability,
  getNodeStatus,
  loadNodePreferences,
  saveNodePreferences,
  selectDataDirectory,
  selectDiagnosticOutput,
  selectNodeBinary,
  selectReleaseArchive,
  startNode,
  stopNode,
  validateNodeBinary,
  verifyApprovedReleaseArchive,
} from '../lib/desktop'
import { LiveDagPage } from '../pages/LiveDagPage'
import { LogsPage } from '../pages/LogsPage'
import { NetworkPage } from '../pages/NetworkPage'
import { NodePage } from '../pages/NodePage'
import { OverviewPage } from '../pages/OverviewPage'
import { SettingsPage } from '../pages/SettingsPage'
import type {
  AppSection,
  BinaryInfo,
  BinaryProvenance,
  DesktopBridgeStatus,
  DiagnosticExportResult,
  LogEntry,
  NodeObservability,
  NodePreferences,
  NodeRuntimeStatus,
  ReleaseVerification,
  RpcHealth,
} from '../types'

const sectionTitles: Record<AppSection, { eyebrow: string; title: string }> = {
  overview: { eyebrow: 'PulseDAG operator workspace', title: 'Overview' },
  node: { eyebrow: 'Local process supervision', title: 'Node' },
  network: { eyebrow: 'Peer and synchronization intelligence', title: 'Network' },
  dag: { eyebrow: 'Realtime consensus activity', title: 'Live DAG' },
  logs: { eyebrow: 'Local diagnostics', title: 'Logs' },
  settings: { eyebrow: 'Desktop preferences', title: 'Settings' },
}

const initialNodeStatus: NodeRuntimeStatus = {
  running: false,
  pid: null,
  startedAtMs: null,
  uptimeSeconds: null,
  lastExitCode: null,
  executablePath: null,
}

const initialRpcHealth: RpcHealth = {
  reachable: false,
  statusCode: null,
  latencyMs: 0,
  message: 'Not checked yet',
}

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function App() {
  const [section, setSection] = useState<AppSection>('overview')
  const [theme, setTheme] = useState<'dark' | 'light'>(() => (localStorage.getItem('pulsedag.desktop.theme') === 'light' ? 'light' : 'dark'))
  const [bridge, setBridge] = useState<DesktopBridgeStatus | null>(null)
  const [preferences, setPreferences] = useState<NodePreferences>(() => loadNodePreferences())
  const [binaryInfo, setBinaryInfo] = useState<BinaryInfo | null>(null)
  const [releaseVerification, setReleaseVerification] = useState<ReleaseVerification | null>(null)
  const [binaryProvenance, setBinaryProvenance] = useState<BinaryProvenance | null>(null)
  const [diagnosticExport, setDiagnosticExport] = useState<DiagnosticExportResult | null>(null)
  const [nodeStatus, setNodeStatus] = useState<NodeRuntimeStatus>(initialNodeStatus)
  const [rpcHealth, setRpcHealth] = useState<RpcHealth>(initialRpcHealth)
  const [observability, setObservability] = useState<NodeObservability | null>(null)
  const [observabilityError, setObservabilityError] = useState('')
  const [observabilityLoading, setObservabilityLoading] = useState(false)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [busy, setBusy] = useState(false)
  const [operationError, setOperationError] = useState('')
  const logCursor = useRef(0)

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem('pulsedag.desktop.theme', theme)
  }, [theme])

  useEffect(() => {
    void Promise.all([
      getDesktopBridgeStatus().then(setBridge),
      getBinaryProvenance().then(setBinaryProvenance),
    ])
  }, [])

  const refreshRuntime = useCallback(async () => {
    const [nextStatus, nextRpc] = await Promise.all([
      getNodeStatus(),
      checkRpcHealth(preferences.rpcEndpoint),
    ])
    setNodeStatus(nextStatus)
    setRpcHealth(nextRpc)
  }, [preferences.rpcEndpoint])

  const refreshObservability = useCallback(async () => {
    setObservabilityLoading(true)
    try {
      const snapshot = await getNodeObservability(preferences.rpcEndpoint)
      setObservability(snapshot)
      setObservabilityError('')
    } catch (error) {
      setObservabilityError(readableError(error))
    } finally {
      setObservabilityLoading(false)
    }
  }, [preferences.rpcEndpoint])

  useEffect(() => {
    void refreshRuntime()
    const timer = window.setInterval(() => void refreshRuntime(), 2_000)
    return () => window.clearInterval(timer)
  }, [refreshRuntime])

  useEffect(() => {
    if (!rpcHealth.reachable) {
      setObservabilityError(nodeStatus.running
        ? 'The local RPC is not ready for read-only observability yet.'
        : 'Start the local node to load network and DAG data.')
      return
    }

    void refreshObservability()
    const timer = window.setInterval(() => void refreshObservability(), 5_000)
    return () => window.clearInterval(timer)
  }, [nodeStatus.running, refreshObservability, rpcHealth.reachable])

  useEffect(() => {
    async function pollLogs() {
      const batch = await getNodeLogs(logCursor.current, 300)
      if (batch.entries.length > 0) {
        logCursor.current = batch.nextCursor
        setLogs((current) => [...current, ...batch.entries].slice(-2_000))
      }
    }

    void pollLogs()
    const timer = window.setInterval(() => void pollLogs(), nodeStatus.running ? 800 : 2_000)
    return () => window.clearInterval(timer)
  }, [nodeStatus.running])

  async function perform(action: () => Promise<void>) {
    setBusy(true)
    setOperationError('')
    try {
      await action()
    } catch (error) {
      setOperationError(readableError(error))
    } finally {
      setBusy(false)
      await refreshRuntime()
    }
  }

  async function handleValidate(path = preferences.executablePath): Promise<BinaryInfo | null> {
    try {
      const info = await validateNodeBinary(path)
      setBinaryInfo(info)
      if (binaryProvenance && !binaryProvenance.selectedBinarySha256.toLowerCase().startsWith(info.sha256.toLowerCase())) {
        setBinaryProvenance(null)
      }
      setOperationError('')
      return info
    } catch (error) {
      setBinaryInfo(null)
      setBinaryProvenance(null)
      setOperationError(readableError(error))
      return null
    }
  }

  async function handleDiscover(): Promise<BinaryInfo | null> {
    try {
      const info = await discoverNodeBinary()
      if (!info) {
        setOperationError('pulsedagd was not found beside the application, in ./bin or on PATH.')
        return null
      }
      const next = { ...preferences, executablePath: info.path }
      setPreferences(next)
      saveNodePreferences(next)
      setBinaryInfo(info)
      setBinaryProvenance(null)
      setOperationError('')
      return info
    } catch (error) {
      setOperationError(readableError(error))
      return null
    }
  }

  async function handlePickExecutable(): Promise<string | null> {
    try {
      const path = await selectNodeBinary()
      setOperationError('')
      return path
    } catch (error) {
      setOperationError(readableError(error))
      return null
    }
  }

  async function handlePickDataDirectory(): Promise<string | null> {
    try {
      const path = await selectDataDirectory()
      setOperationError('')
      return path
    } catch (error) {
      setOperationError(readableError(error))
      return null
    }
  }

  async function handleVerifyRelease(): Promise<ReleaseVerification | null> {
    try {
      setOperationError('')
      const path = await selectReleaseArchive()
      if (!path) return null
      const verification = await verifyApprovedReleaseArchive(path)
      setReleaseVerification(verification)
      setBinaryProvenance(null)
      setOperationError(verification.approved ? '' : verification.message)
      return verification
    } catch (error) {
      setReleaseVerification(null)
      setBinaryProvenance(null)
      setOperationError(readableError(error))
      return null
    }
  }

  async function handleBindProvenance(executablePath: string): Promise<BinaryProvenance | null> {
    if (!releaseVerification?.approved) {
      setOperationError('Verify an approved release archive before linking the executable.')
      return null
    }
    try {
      setOperationError('')
      const proof = await bindBinaryToVerifiedArchive(
        releaseVerification.archivePath,
        executablePath,
      )
      setBinaryProvenance(proof.approved ? proof : null)
      setOperationError(proof.approved ? '' : proof.message)
      return proof
    } catch (error) {
      setBinaryProvenance(null)
      setOperationError(readableError(error))
      return null
    }
  }

  function savePreferences(next: NodePreferences) {
    const pathChanged = next.executablePath !== preferences.executablePath
    const endpointChanged = next.rpcEndpoint !== preferences.rpcEndpoint
    saveNodePreferences(next)
    setPreferences(next)
    if (pathChanged) {
      setBinaryInfo(null)
      setBinaryProvenance(null)
    }
    if (endpointChanged) {
      setObservability(null)
      setObservabilityError('')
    }
    setOperationError('')
  }

  function handleStart() {
    void perform(async () => {
      saveNodePreferences(preferences)
      const info = await validateNodeBinary(preferences.executablePath)
      setBinaryInfo(info)
      setNodeStatus(await startNode(preferences))
    })
  }

  function handleStop() {
    void perform(async () => setNodeStatus(await stopNode()))
  }

  function handleRestart() {
    void perform(async () => {
      await stopNode()
      setNodeStatus(await startNode(preferences))
    })
  }

  function handleClearLogs() {
    void perform(async () => {
      await clearNodeLogs()
      logCursor.current = 0
      setLogs([])
      setDiagnosticExport(null)
    })
  }

  function handleExportDiagnostics() {
    void perform(async () => {
      const outputPath = await selectDiagnosticOutput()
      if (!outputPath) return
      setDiagnosticExport(await exportDiagnostics(outputPath, preferences, rpcHealth))
    })
  }

  function handleRefreshAll() {
    void Promise.all([refreshRuntime(), refreshObservability()])
  }

  const heading = sectionTitles[section]
  const configured = preferences.executablePath.trim().length > 0 && preferences.dataDirectory.trim().length > 0
  const online = nodeStatus.running && rpcHealth.reachable
  const connectionLabel = online ? 'Node online' : nodeStatus.running ? 'Node starting' : 'Node offline'

  let content
  if (section === 'overview') {
    content = (
      <OverviewPage
        bridge={bridge}
        preferences={preferences}
        binaryInfo={binaryInfo}
        nodeStatus={nodeStatus}
        rpcHealth={rpcHealth}
        busy={busy}
        onOpenSettings={() => setSection('settings')}
        onStart={handleStart}
      />
    )
  } else if (section === 'node') {
    content = (
      <NodePage
        preferences={preferences}
        binaryInfo={binaryInfo}
        nodeStatus={nodeStatus}
        rpcHealth={rpcHealth}
        busy={busy}
        error={operationError}
        onStart={handleStart}
        onStop={handleStop}
        onRestart={handleRestart}
        onValidate={() => void handleValidate()}
        onOpenSettings={() => setSection('settings')}
      />
    )
  } else if (section === 'settings') {
    content = (
      <SettingsPage
        value={preferences}
        binaryInfo={binaryInfo}
        releaseVerification={releaseVerification}
        binaryProvenance={binaryProvenance}
        busy={busy}
        error={operationError}
        onSave={savePreferences}
        onDetect={handleDiscover}
        onValidate={handleValidate}
        onPickExecutable={handlePickExecutable}
        onPickDataDirectory={handlePickDataDirectory}
        onVerifyRelease={handleVerifyRelease}
        onBindProvenance={handleBindProvenance}
      />
    )
  } else if (section === 'logs') {
    content = (
      <LogsPage
        entries={logs}
        running={nodeStatus.running}
        busy={busy}
        error={operationError}
        exportResult={diagnosticExport}
        onClear={handleClearLogs}
        onExport={handleExportDiagnostics}
      />
    )
  } else if (section === 'network') {
    content = (
      <NetworkPage
        snapshot={observability}
        loading={observabilityLoading}
        error={observabilityError}
        rpcReachable={rpcHealth.reachable}
        onRefresh={() => void refreshObservability()}
      />
    )
  } else {
    content = (
      <LiveDagPage
        snapshot={observability}
        loading={observabilityLoading}
        error={observabilityError}
        rpcReachable={rpcHealth.reachable}
        onRefresh={() => void refreshObservability()}
      />
    )
  }

  const showGlobalOperationError = operationError
    && !['node', 'settings', 'logs'].includes(section)

  return (
    <div className="app-shell">
      <Sidebar active={section} onChange={setSection} />
      <main>
        <header className="topbar">
          <div><span className="eyebrow">{heading.eyebrow}</span><h1>{heading.title}</h1></div>
          <div className="topbar-actions">
            <span className={`sync-pill ${online ? 'online' : 'warning'}`}><i />{connectionLabel}{nodeStatus.pid ? ` · PID ${nodeStatus.pid}` : ''}</span>
            <button className="icon-button" onClick={handleRefreshAll} aria-label="Refresh node status and read-only data">↻</button>
            <button className="icon-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} aria-label="Toggle theme">{theme === 'dark' ? '☼' : '◐'}</button>
          </div>
        </header>
        {showGlobalOperationError && <div className="notice notice-error">{operationError}</div>}
        {!configured && section !== 'settings' && <div className="notice notice-warning">Select pulsedagd and a persistent data directory before starting the node.</div>}
        {content}
      </main>
    </div>
  )
}
