import { useEffect, useState } from 'react'
import { getBinaryProvenance } from '../lib/desktop'
import type { BinaryInfo, BinaryProvenance, NodePreferences, NodeRuntimeStatus, RpcHealth } from '../types'

const V24_FINAL_SOURCE = '876b48826a3875b729888edb88e2b0eea15bb717'

type NodePageProps = {
  preferences: NodePreferences
  binaryInfo: BinaryInfo | null
  nodeStatus: NodeRuntimeStatus
  rpcHealth: RpcHealth
  busy: boolean
  error: string
  onStart: () => void
  onStop: () => void
  onRestart: () => void
  onValidate: () => void
  onOpenSettings: () => void
}

function isFinalV24NodeProvenance(proof: BinaryProvenance | null): boolean {
  return Boolean(
    proof?.approved
      && proof.releaseTag === 'v2.4.0'
      && proof.sourceCommit === V24_FINAL_SOURCE
      && proof.archiveName.startsWith('pulsedagd-v2.4.0-'),
  )
}

export function NodePage({ preferences, binaryInfo, nodeStatus, rpcHealth, busy, error, onStart, onStop, onRestart, onValidate, onOpenSettings }: NodePageProps) {
  const [binaryProvenance, setBinaryProvenance] = useState<BinaryProvenance | null>(null)
  const configured = preferences.executablePath.trim().length > 0 && preferences.dataDirectory.trim().length > 0
  const privateMode = preferences.configProfile === 'private'
  const privateProvenanceReady = !privateMode || isFinalV24NodeProvenance(binaryProvenance)
  const startBlocked = busy || (!nodeStatus.running && (!configured || !privateProvenanceReady))

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

  return (
    <>
      {error && <div className="notice notice-error">{error}</div>}
      {privateMode && !privateProvenanceReady && !nodeStatus.running && (
        <div className="notice notice-warning">
          Private v2.4.0 stays locked until pulsedagd is verified and linked byte-for-byte to the final Task31 release in this desktop session.
          <div className="button-row field-actions">
            <button className="secondary-button compact-button" type="button" onClick={onOpenSettings} disabled={busy}>Verify &amp; link v2.4 release</button>
          </div>
        </div>
      )}
      <section className="page-grid">
        <article className="panel node-control-panel">
          <div className="panel-header">
            <div><span className="eyebrow">Process control</span><h3>PulseDAG node</h3></div>
            <span className={`status-badge ${nodeStatus.running ? 'success' : 'warning'}`}>{nodeStatus.running ? 'Running' : 'Stopped'}</span>
          </div>
          <div className="node-control-body">
            <div className="node-avatar"><span>PD</span></div>
            <div>
              <strong>{preferences.executablePath || 'No executable selected'}</strong>
              <small>{preferences.rpcEndpoint} · {preferences.configProfile}</small>
            </div>
          </div>
          <div className="button-row">
            <button className="primary-button" onClick={nodeStatus.running ? onStop : onStart} disabled={startBlocked}>{busy ? 'Working…' : nodeStatus.running ? 'Stop node' : privateMode && !privateProvenanceReady ? 'Link release before start' : 'Start node'}</button>
            <button className="secondary-button" onClick={onRestart} disabled={busy || !nodeStatus.running}>Restart</button>
            <button className="secondary-button" onClick={onValidate} disabled={busy || !preferences.executablePath}>Validate binary</button>
            <button className="secondary-button" onClick={onOpenSettings}>Configure</button>
          </div>
        </article>

        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Runtime</span><h3>Supervision</h3></div></div>
          <div className="detail-list">
            <div><span>PID</span><strong>{nodeStatus.pid ?? '—'}</strong></div>
            <div><span>Uptime</span><strong>{nodeStatus.uptimeSeconds != null ? `${nodeStatus.uptimeSeconds}s` : '—'}</strong></div>
            <div><span>Last exit</span><strong>{nodeStatus.lastExitCode ?? '—'}</strong></div>
            <div><span>RPC health</span><strong className={rpcHealth.reachable ? 'success-text' : 'warning-text'}>{rpcHealth.reachable ? `${rpcHealth.latencyMs} ms` : 'Unavailable'}</strong></div>
          </div>
        </article>
      </section>

      <section className="page-grid lower-grid">
        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Executable evidence</span><h3>Local binary</h3></div></div>
          <div className="detail-list">
            <div><span>Validated</span><strong>{binaryInfo ? 'Yes' : 'Not in this session'}</strong></div>
            <div><span>Private provenance</span><strong className={privateMode ? privateProvenanceReady ? 'success-text' : 'warning-text' : ''}>{privateMode ? privateProvenanceReady ? 'Final v2.4 linked' : 'Link required' : 'Not required'}</strong></div>
            <div><span>Size</span><strong>{binaryInfo ? `${(binaryInfo.sizeBytes / 1_048_576).toFixed(2)} MiB` : '—'}</strong></div>
            <div className="hash-row"><span>SHA-256</span><strong>{binaryInfo?.sha256 ?? '—'}</strong></div>
          </div>
        </article>
        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Persistent state</span><h3>Data boundary</h3></div></div>
          <div className="detail-list">
            <div><span>Data directory</span><strong>{preferences.dataDirectory || '—'}</strong></div>
            <div><span>RocksDB</span><strong>{preferences.dataDirectory ? `${preferences.dataDirectory}/rocksdb` : '—'}</strong></div>
            <div><span>Identity</span><strong>{preferences.dataDirectory ? `${preferences.dataDirectory}/identity.key` : '—'}</strong></div>
          </div>
        </article>
      </section>
    </>
  )
}
