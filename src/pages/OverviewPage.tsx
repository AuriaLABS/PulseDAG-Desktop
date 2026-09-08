import { useEffect, useState } from 'react'
import { StatusCard } from '../components/StatusCard'
import { getBinaryProvenance } from '../lib/desktop'
import type { BinaryInfo, BinaryProvenance, DesktopBridgeStatus, NodePreferences, NodeRuntimeStatus, RpcHealth } from '../types'

const V24_FINAL_SOURCE = '876b48826a3875b729888edb88e2b0eea15bb717'

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

function isFinalV24NodeProvenance(proof: BinaryProvenance | null): boolean {
  return Boolean(
    proof?.approved
      && proof.releaseTag === 'v2.4.0'
      && proof.sourceCommit === V24_FINAL_SOURCE
      && proof.archiveName.startsWith('pulsedagd-v2.4.0-'),
  )
}

export function OverviewPage({ bridge, preferences, binaryInfo, nodeStatus, rpcHealth, busy, onOpenSettings, onStart }: OverviewPageProps) {
  const [binaryProvenance, setBinaryProvenance] = useState<BinaryProvenance | null>(null)
  const configured = preferences.executablePath.trim().length > 0 && preferences.dataDirectory.trim().length > 0
  const online = nodeStatus.running && rpcHealth.reachable
  const privateMode = preferences.configProfile === 'private'
  const privateProvenanceReady = !privateMode || isFinalV24NodeProvenance(binaryProvenance)
  const readiness = [true, Boolean(binaryInfo), rpcHealth.reachable, nodeStatus.running].filter(Boolean).length

  useEffect(() => {
    let cancelled = false
    void getBinaryProvenance()
      .then((proof) => {
        if (!cancelled) setBinaryProvenance(proof)
      })
      .catch(() => {
        if (!cancelled) setBinaryProvenance(null)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const primaryAction = nodeStatus.running || (privateMode && !privateProvenanceReady) ? onOpenSettings : onStart
  const primaryLabel = nodeStatus.running
    ? 'Review configuration'
    : privateMode && !privateProvenanceReady
      ? 'Verify & link v2.4 release'
      : 'Start node'

  return (
    <>
      {privateMode && !privateProvenanceReady && !nodeStatus.running && (
        <div className="notice notice-warning">Private v2.4.0 launch is locked until the final Task31 node archive is verified and pulsedagd is linked byte-for-byte in this desktop session.</div>
      )}
      <section className="hero-panel">
        <div>
          <span className="eyebrow">Local node control</span>
          <h2>Your PulseDAG node,<br />from one secure desktop.</h2>
          <p>Validate the exact node executable, supervise the local process and inspect its loopback health without exposing administrative RPC to a browser.</p>
          <div className="hero-actions">
            <button className="primary-button" onClick={primaryAction} disabled={busy || (!configured && !nodeStatus.running && privateProvenanceReady)}>{primaryLabel}</button>
            <button className="secondary-button" onClick={onOpenSettings}>{configured ? 'Settings' : 'Configure node'}</button>
          </div>
        </div>
        <div className="node-orbit" aria-label={`Node ${online ? 'online' : nodeStatus.running ? 'starting' : 'offline'} illustration`}>
          <div className="orbit orbit-a"><i /><i /><i /></div>
          <div className="orbit orbit-b"><i /><i /></div>
          <div className={`orbit-core ${online ? 'online' : ''}`}><span>{online ? 'ONLINE' : nodeStatus.running ? 'STARTING' : 'OFFLINE'}</span><small>{nodeStatus.running ? `PID ${nodeStatus.pid ?? '—'}` : privateMode && !privateProvenanceReady ? 'Release link required' : 'Awaiting start'}</small></div>
        </div>
      </section>

      <section className="metrics-grid" aria-label="Node summary">
        <StatusCard label="Node process" value={nodeStatus.running ? 'Running' : 'Stopped'} detail={nodeStatus.uptimeSeconds != null ? `${nodeStatus.uptimeSeconds}s uptime` : `Exit ${nodeStatus.lastExitCode ?? '—'}`} tone={nodeStatus.running ? 'success' : 'warning'} />
        <StatusCard label="RPC connection" value={rpcHealth.reachable ? 'Healthy' : 'Offline'} detail={rpcHealth.reachable ? `${rpcHealth.latencyMs} ms · HTTP ${rpcHealth.statusCode}` : rpcHealth.message} tone={rpcHealth.reachable ? 'success' : 'warning'} />
        <StatusCard label="Profile" value={preferences.configProfile} detail={privateMode ? privateProvenanceReady ? 'Final v2.4 provenance linked' : 'Final v2.4 provenance required' : preferences.rpcEndpoint} tone={privateMode && !privateProvenanceReady ? 'warning' : 'neutral'} />
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
            <div className={nodeStatus.running ? 'complete' : ''}><i>{nodeStatus.running ? '✓' : '4'}</i><span><strong>Start supervised node</strong><small>{privateMode && !privateProvenanceReady ? 'Verify and link final v2.4 provenance first' : 'Process, output capture and safe stop'}</small></span></div>
          </div>
        </article>

        <article className="panel boundary-panel">
          <div className="panel-header"><div><span className="eyebrow">Security boundary</span><h3>Local by default</h3></div></div>
          <p>All inherited PulseDAG environment variables are removed before launch. The desktop backend supplies an explicit local configuration and forces administrative RPC off.</p>
          <ul>
            <li>RPC origin must be localhost</li>
            <li>No operator token reaches the frontend</li>
            <li>Private v2.4 requires final release provenance in-session</li>
            <li>No wallet or signing controls</li>
          </ul>
        </article>
      </section>
    </>
  )
}
