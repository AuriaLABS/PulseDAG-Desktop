import { StatusCard } from '../components/StatusCard'
import type { DesktopBridgeStatus, NodePreferences } from '../types'

type OverviewPageProps = {
  bridge: DesktopBridgeStatus | null
  preferences: NodePreferences
  onOpenSettings: () => void
}

export function OverviewPage({ bridge, preferences, onOpenSettings }: OverviewPageProps) {
  const configured = preferences.executablePath.trim().length > 0

  return (
    <>
      <section className="hero-panel">
        <div>
          <span className="eyebrow">Local node control</span>
          <h2>Your PulseDAG node,<br />from one secure desktop.</h2>
          <p>Configure a local node executable, inspect its health and prepare the operator workflow without exposing administrative RPC to a browser.</p>
          <div className="hero-actions">
            <button className="primary-button" onClick={onOpenSettings}>{configured ? 'Review configuration' : 'Configure node'}</button>
            <button className="secondary-button" disabled>Start node</button>
          </div>
        </div>
        <div className="node-orbit" aria-label="Node offline illustration">
          <div className="orbit orbit-a"><i /><i /><i /></div>
          <div className="orbit orbit-b"><i /><i /></div>
          <div className="orbit-core"><span>OFFLINE</span><small>Awaiting setup</small></div>
        </div>
      </section>

      <section className="metrics-grid" aria-label="Node summary">
        <StatusCard label="Node process" value={bridge?.nodeRunning ? 'Running' : 'Stopped'} detail="Process supervision not enabled yet" tone="warning" />
        <StatusCard label="RPC connection" value={bridge?.rpcReachable ? 'Reachable' : 'Offline'} detail={preferences.rpcEndpoint} tone="warning" />
        <StatusCard label="Network" value={preferences.network.replace('-', ' ')} detail="Configured locally" />
        <StatusCard label="Desktop bridge" value={bridge ? 'Ready' : 'Connecting'} detail={bridge ? `${bridge.platform} · ${bridge.appVersion}` : 'Loading Tauri status'} tone={bridge ? 'success' : 'neutral'} />
      </section>

      <section className="dashboard-grid">
        <article className="panel readiness-panel">
          <div className="panel-header">
            <div><span className="eyebrow">Readiness</span><h3>First-run checklist</h3></div>
            <span className="progress-pill">1 / 4</span>
          </div>
          <div className="checklist">
            <div className="complete"><i>✓</i><span><strong>Desktop shell</strong><small>Tauri bridge and application layout</small></span></div>
            <div className={configured ? 'complete' : ''}><i>{configured ? '✓' : '2'}</i><span><strong>Choose pulsedagd</strong><small>Local executable path and network</small></span></div>
            <div><i>3</i><span><strong>Validate RPC</strong><small>Confirm loopback endpoint and API version</small></span></div>
            <div><i>4</i><span><strong>Start supervised node</strong><small>Launch, stop and restart safely</small></span></div>
          </div>
        </article>

        <article className="panel boundary-panel">
          <div className="panel-header"><div><span className="eyebrow">Security boundary</span><h3>Local by default</h3></div></div>
          <p>Operator credentials and process controls remain inside the Tauri backend. The frontend receives only explicit status data and approved commands.</p>
          <ul>
            <li>No remote administrative RPC exposure</li>
            <li>No wallet or signing in this milestone</li>
            <li>No mining controls in this milestone</li>
          </ul>
        </article>
      </section>
    </>
  )
}
