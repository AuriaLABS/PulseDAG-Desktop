import { useCallback, useEffect, useRef, useState } from 'react'
import { Sidebar } from '../components/Sidebar'
import {
  checkRpcHealth,
  clearNodeLogs,
  discoverNodeBinary,
  getDesktopBridgeStatus,
  getNodeLogs,
  getNodeStatus,
  loadNodePreferences,
  saveNodePreferences,
  startNode,
  stopNode,
  validateNodeBinary,
} from '../lib/desktop'
import { LogsPage } from '../pages/LogsPage'
import { NodePage } from '../pages/NodePage'
import { OverviewPage } from '../pages/OverviewPage'
import { PlaceholderPage } from '../pages/PlaceholderPage'
import { SettingsPage } from '../pages/SettingsPage'
import type {
  AppSection,
  BinaryInfo,
  DesktopBridgeStatus,
  LogEntry,
  NodePreferences,
  NodeRuntimeStatus,
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
  const [nodeStatus, setNodeStatus] = useState<NodeRuntimeStatus>(initialNodeStatus)
  const [rpcHealth, setRpcHealth] = useState<RpcHealth>(initialRpcHealth)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [busy, setBusy] = useState(false)
  const [operationError, setOperationError] = useState('')
  const logCursor = useRef(0)

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem('pulsedag.desktop.theme', theme)
  }, [theme])

  useEffect(() => {
    void getDesktopBridgeStatus().then(setBridge)
  }, [])

  const refreshRuntime = useCallback(async () => {
    const [nextStatus, nextRpc] = await Promise.all([
      getNodeStatus(),
      checkRpcHealth(preferences.rpcEndpoint),
    ])
    setNodeStatus(nextStatus)
    setRpcHealth(nextRpc)
  }, [preferences.rpcEndpoint])

  useEffect(() => {
    void refreshRuntime()
    const timer = window.setInterval(() => void refreshRuntime(), 2_000)
    return () => window.clearInterval(timer)
  }, [refreshRuntime])

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
      setOperationError('')
      return info
    } catch (error) {
      setBinaryInfo(null)
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
      setOperationError('')
      return info
    } catch (error) {
      setOperationError(readableError(error))
      return null
    }
  }

  function savePreferences(next: NodePreferences) {
    const pathChanged = next.executablePath !== preferences.executablePath
    saveNodePreferences(next)
    setPreferences(next)
    if (pathChanged) setBinaryInfo(null)
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
    })
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
        busy={busy}
        error={operationError}
        onSave={savePreferences}
        onDetect={handleDiscover}
        onValidate={handleValidate}
      />
    )
  } else if (section === 'logs') {
    content = <LogsPage entries={logs} running={nodeStatus.running} onClear={handleClearLogs} />
  } else if (section === 'network') {
    content = <PlaceholderPage eyebrow="Network observability" title="Peers and synchronization" description="This area will consume approved local status endpoints and present sync lag, peer health and connection history." items={['Peer table', 'Sync progress', 'Connection health', 'Network identity']} />
  } else {
    content = <PlaceholderPage eyebrow="Consensus visualization" title="Live DAG workspace" description="The explorer DAG components will be adapted for local node data without exposing operator controls to the public web client." items={['Realtime block graph', 'Tip selection', 'Block inspector', 'Performance timeline']} />
  }

  return (
    <div className="app-shell">
      <Sidebar active={section} onChange={setSection} />
      <main>
        <header className="topbar">
          <div><span className="eyebrow">{heading.eyebrow}</span><h1>{heading.title}</h1></div>
          <div className="topbar-actions">
            <span className={`sync-pill ${online ? 'online' : 'warning'}`}><i />{connectionLabel}{nodeStatus.pid ? ` · PID ${nodeStatus.pid}` : ''}</span>
            <button className="icon-button" onClick={() => void refreshRuntime()} aria-label="Refresh node status">↻</button>
            <button className="icon-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} aria-label="Toggle theme">{theme === 'dark' ? '☼' : '◐'}</button>
          </div>
        </header>
        {!configured && section !== 'settings' && <div className="notice notice-warning">Select pulsedagd and a persistent data directory before starting the node.</div>}
        {content}
      </main>
    </div>
  )
}
