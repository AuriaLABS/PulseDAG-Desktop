import { StatusCard } from '../components/StatusCard'
import type { BinaryInfo, DesktopBridgeStatus, NodePreferences, NodeRuntimeStatus, RpcHealth } from '../types'

type OverviewPageProps = {
  bridge: DesktopBridgeStatus | null
  preferences: NodePreferences
  binaryInfo: BinaryInfo | null
  nodeStatus: NodeRuntimeStatus
  rpcHealth: RpcHealth
  busy: boolean
  onOpenSettings: () => void
  onStart: () => void
}

export function OverviewPage({ bridge, preferences, binaryInfo, nodeStatus, rpcHealth, busy, onOpenSettings, onStart }: OverviewPageProps) {
  const configured = preferences.executablePath.trim().length > 0 && preferences.dataDirectory.trim().length > 0
  const online = nodeStatus.running && rpcHealth.reachable
  const readiness = [true, Boolean(binaryInfo), rpcHealth.reachable, nodeStatus.running].filter(Boolean).length

  return (
    <>
      <section className="hero-panel">
        <div>
          <span className="eyebrow">Local node control</span>
          <h2>Your PulseDAG node,<br />from one secure desktop.</h2>
          <p>Validate the exact node executable, supervise the local process and inspect its loopback health without exposing administrative RPC to a browser.</p>
          <div className="hero-actions">
            <button className="primary-button" onClick={nodeStatus.running ? onOpenSettings : onStart} disabled={busy || (!configured && !nodeStatus.running)}>{nodeStatus.running ? 'Review configuration' : 'Start node'}</button>
            <button className="secondary-button" onClick={onOpenSettings}>{configured ? 'Settings' : 'Configure node'}</button>
          </div>
        </div>
        <div className="node-orbit" aria-label={`Node ${online ? 'online' : nodeStatus.running ? 'starting' : 'offline'} illustration`}>
          <div className="orbit orbit-a"><i /><i /><i /></div>
          <div className="orbit orbit-b"><i /><i /></div>
          <div className={`orbit-core ${online ? 'online' : ''}`}><span>{online ? 'ONLINE' : nodeStatus.running ? 'STARTING' : 'OFFLINE'}</span><small>{nodeStatus.running ? `PID ${nodeStatus.pid ?? '—'}` : 'Awaiting start'}</small></div>
        </div>
      </section>

      <section className="metrics-grid" aria-label="Node summary">
        <StatusCard label="Node process" value={nodeStatus.running ? 'Running' : 'Stopped'} detail={nodeStatus.uptimeSeconds != null ? `${nodeStatus.uptimeSeconds}s uptime` : `Exit ${nodeStatus.lastExitCode ?? '—'}`} tone={nodeStatus.running ? 'success' : 'warning'} />
        <StatusCard label="RPC connection" value={rpcHealth.reachable ? 'Healthy' : 'Offline'} detail={rpcHealth.reachable ? `${rpcHealth.latencyMs} ms · HTTP ${rpcHealth.statusCode}` : rpcHealth.message} tone={rpcHealth.reachable ? 'success' : 'warning'} />
        <StatusCard label="Profile" value={preferences.configProfile} detail={preferences.rpcEndpoint} />
        <StatusCard label="Desktop bridge" value={bridge ? 'Ready' : 'Connecting'} detail={bridge ? `${bridge.platform} · ${bridge.appVersion}` : 'Loading Tauri status'} tone={bridge ? 'success' : 'neutral'} />
      </section>

      <section className="dashboard-grid">
        <article className="panel readiness-panel">
          <div className="panel-header">
            <div><span className="eyebrow">Readiness</span><h3>First-run checklist</h3></div>
            <span className="progress-pill">{readiness} / 4</span>
          </div>
          <div className="checklist">
            <div className="complete"><i>✓</i><span><strong>Desktop shell</strong><small>Tauri bridge and application layout</small></span></div>
            <div className={binaryInfo ? 'complete' : ''}><i>{binaryInfo ? '✓' : '2'}</i><span><strong>Validate pulsedagd</strong><small>{binaryInfo ? binaryInfo.sha256.slice(0, 16) : 'Name, permissions and SHA-256'}</small></span></div>
            <div className={rpcHealth.reachable ? 'complete' : ''}><i>{rpcHealth.reachable ? '✓' : '3'}</i><span><strong>Validate RPC</strong><small>Loopback-only /health probe</small></span></div>
            <div className={nodeStatus.running ? 'complete' : ''}><i>{nodeStatus.running ? '✓' : '4'}</i><span><strong>Start supervised node</strong><small>Process, output capture and safe stop</small></span></div>
          </div>
        </article>

        <article className="panel boundary-panel">
          <div className="panel-header"><div><span className="eyebrow">Security boundary</span><h3>Local by default</h3></div></div>
          <p>All inherited PulseDAG environment variables are removed before launch. The desktop backend supplies an explicit local configuration and forces administrative RPC off.</p>
          <ul>
            <li>RPC origin must be localhost</li>
            <li>No operator token reaches the frontend</li>
            <li>No wallet, signing or mining controls</li>
          </ul>
        </article>
      </section>
    </>
  )
}
