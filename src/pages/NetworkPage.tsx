import { StatusCard } from '../components/StatusCard'
import type { NodeObservability } from '../types'

const number = new Intl.NumberFormat('en-US')

type NetworkPageProps = {
  snapshot: NodeObservability | null
  loading: boolean
  error: string
  rpcReachable: boolean
  onRefresh: () => void
}

function freshness(snapshot: NodeObservability | null): string {
  if (!snapshot) return 'No snapshot'
  return new Date(snapshot.fetchedAtMs).toLocaleTimeString()
}

export function NetworkPage({ snapshot, loading, error, rpcReachable, onRefresh }: NetworkPageProps) {
  const status = snapshot?.status
  const sync = snapshot?.sync
  const mempool = snapshot?.mempool
  const pow = snapshot?.pow
  const degraded = Boolean(
    status?.rpcResponseDegraded
    || status?.rpcResponseStale
    || sync?.rpcResponseDegraded
    || sync?.rpcResponseStale
    || (sync && !sync.consistencyOk),
  )

  return (
    <>
      {error && (
        <div className={`notice ${snapshot ? 'notice-warning' : 'notice-error'}`} role="alert">
          {snapshot ? `Showing the last valid snapshot. ${error}` : error}
        </div>
      )}
      {snapshot?.warnings.map((warning) => (
        <div className="notice notice-warning" key={warning}>{warning}</div>
      ))}

      <section className="metrics-grid network-metrics" aria-label="Network summary">
        <StatusCard
          label="Peer connections"
          value={status ? number.format(status.peerCount) : '—'}
          detail={status?.p2pMode ?? 'P2P mode unavailable'}
          tone={status && status.peerCount > 0 ? 'success' : 'warning'}
        />
        <StatusCard
          label="Synchronization"
          value={sync?.syncState ?? status?.syncState ?? '—'}
          detail={sync ? `${number.format(sync.lagBlocks)} block lag` : 'Detailed sync endpoint unavailable'}
          tone={sync && sync.lagBlocks === 0 && sync.consistencyOk ? 'success' : 'warning'}
        />
        <StatusCard
          label="Selected height"
          value={status?.selectedHeight == null ? '—' : number.format(status.selectedHeight)}
          detail={status ? `${number.format(status.bestHeight)} best height` : 'No node snapshot'}
        />
        <StatusCard
          label="RPC snapshot"
          value={!snapshot ? 'Offline' : degraded ? 'Degraded' : 'Current'}
          detail={`${freshness(snapshot)}${snapshot ? ` · ${number.format(snapshot.latencyMs)} ms` : ''}`}
          tone={!snapshot || degraded ? 'warning' : 'success'}
        />
      </section>

      <section className="network-observability-grid">
        <article className="panel">
          <div className="panel-header">
            <div><span className="eyebrow">Chain identity</span><h3>Connected node</h3></div>
            <button className="secondary-button compact-button" onClick={onRefresh} disabled={loading || !rpcReachable}>
              {loading ? 'Refreshing…' : 'Refresh'}
            </button>
          </div>
          <div className="detail-list">
            <div><span>Service</span><strong>{status?.service ?? '—'}</strong></div>
            <div><span>Version</span><strong>{status?.version ?? '—'}</strong></div>
            <div><span>Network</span><strong>{status?.networkId ?? '—'}</strong></div>
            <div><span>Chain ID</span><strong>{status?.chainId || '—'}</strong></div>
            <div><span>Consensus</span><strong>{status?.consensusMode ?? '—'}</strong></div>
            <div><span>Storage</span><strong>{status?.storageBackend ?? '—'}</strong></div>
          </div>
        </article>

        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Convergence</span><h3>Synchronization</h3></div></div>
          <div className="detail-list">
            <div><span>State</span><strong>{sync?.syncState ?? status?.syncState ?? '—'}</strong></div>
            <div><span>Network height gap</span><strong>{sync ? number.format(sync.networkSelectedHeightGap) : '—'}</strong></div>
            <div><span>Storage replay gap</span><strong>{sync ? number.format(sync.storageReplayGap) : '—'}</strong></div>
            <div><span>Consistency</span><strong className={sync?.consistencyOk ? 'success-text' : 'warning-text'}>{sync ? (sync.consistencyOk ? 'OK' : `${sync.consistencyIssueCount} issue(s)`) : 'Unavailable'}</strong></div>
            <div><span>Live sync errors</span><strong>{sync ? number.format(sync.liveSyncErrorActive) : '—'}</strong></div>
            <div><span>Private rehearsal</span><strong className={sync?.p2pReadyForPrivateRehearsal ? 'success-text' : 'warning-text'}>{sync ? (sync.p2pReadyForPrivateRehearsal ? 'Ready' : 'Not ready') : 'Unavailable'}</strong></div>
          </div>
        </article>
      </section>

      <section className="network-observability-grid lower-grid">
        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Ledger pressure</span><h3>DAG and mempool</h3></div></div>
          <div className="detail-list">
            <div><span>Persisted blocks</span><strong>{status ? number.format(status.persistedBlockCount) : '—'}</strong></div>
            <div><span>DAG tips</span><strong>{status ? number.format(status.tipCount) : '—'}</strong></div>
            <div><span>DAG orphans</span><strong>{status ? number.format(status.orphanCount) : '—'}</strong></div>
            <div><span>Mempool transactions</span><strong>{mempool ? number.format(mempool.transactionCount) : status ? number.format(status.mempoolSize) : '—'}</strong></div>
            <div><span>Mempool orphans</span><strong>{mempool ? number.format(mempool.orphanTransactionCount) : '—'}</strong></div>
            <div><span>Spent outpoints</span><strong>{mempool ? number.format(mempool.spentOutpointsCount) : '—'}</strong></div>
          </div>
        </article>

        <article className="panel">
          <div className="panel-header"><div><span className="eyebrow">Proof of work</span><h3>Cadence health</h3></div></div>
          <div className="detail-list">
            <div><span>Status</span><strong>{pow?.status ?? 'Unavailable'}</strong></div>
            <div><span>Snapshots</span><strong>{pow ? number.format(pow.snapshotCount) : '—'}</strong></div>
            <div><span>Suggested difficulty</span><strong>{pow ? number.format(pow.latestSuggestedDifficulty) : '—'}</strong></div>
            <div><span>Average block interval</span><strong>{pow ? `${pow.latestAvgBlockIntervalSecs.toFixed(2)} s` : '—'}</strong></div>
          </div>
          {pow?.alerts.length ? (
            <div className="readiness-reasons">
              {pow.alerts.map((alert) => <span key={alert}>{alert}</span>)}
            </div>
          ) : null}
        </article>
      </section>

      <article className="panel readiness-detail-panel lower-grid">
        <div className="panel-header"><div><span className="eyebrow">Read-only boundary</span><h3>Peer visibility</h3></div></div>
        <p>The approved v2.3.0 status contract exposes peer count and P2P mode, but not individual peer identities or addresses. PulseDAG Desktop deliberately does not infer or synthesize a peer table.</p>
        {sync?.readinessReasons.length ? (
          <div className="readiness-reasons">
            {sync.readinessReasons.map((reason) => <span key={reason}>{reason}</span>)}
          </div>
        ) : (
          <div className="readiness-reasons"><span>No additional readiness reasons were reported.</span></div>
        )}
      </article>
    </>
  )
}
