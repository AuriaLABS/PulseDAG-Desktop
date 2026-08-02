import type { NodeObservability, RecentDagBlock } from '../types'

const number = new Intl.NumberFormat('en-US')

type LiveDagPageProps = {
  snapshot: NodeObservability | null
  loading: boolean
  error: string
  rpcReachable: boolean
  onRefresh: () => void
}

function timestamp(value: number): string {
  const milliseconds = value < 10_000_000_000 ? value * 1_000 : value
  return new Date(milliseconds).toLocaleString()
}

function shortHash(hash: string): string {
  return hash.length <= 20 ? hash : `${hash.slice(0, 10)}…${hash.slice(-8)}`
}

function blockTone(block: RecentDagBlock, selectedTip: string | null): string {
  return block.hash === selectedTip ? 'selected' : block.parentCount > 1 ? 'merge' : 'linear'
}

export function LiveDagPage({ snapshot, loading, error, rpcReachable, onRefresh }: LiveDagPageProps) {
  const blocks = snapshot?.blocks ?? []
  const selectedTip = snapshot?.status.selectedTip ?? null
  const head = blocks[0] ?? null

  return (
    <>
      {error && (
        <div className={`notice ${snapshot ? 'notice-warning' : 'notice-error'}`} role="alert">
          {snapshot ? `Showing the last valid DAG snapshot. ${error}` : error}
        </div>
      )}

      <section className="dag-live-header panel">
        <div>
          <span className="eyebrow">Approved read-only RPC</span>
          <h2>Recent DAG frontier</h2>
          <p>The recent-block endpoint reports block height, blue score, transaction count, timestamp and parent count. This view does not draw synthetic parent edges when hashes are not provided.</p>
        </div>
        <button className="primary-button" onClick={onRefresh} disabled={loading || !rpcReachable}>
          {loading ? 'Refreshing…' : 'Refresh frontier'}
        </button>
      </section>

      <section className="dag-kpi-strip">
        <div><span>Selected tip</span><strong title={selectedTip ?? ''}>{selectedTip ? shortHash(selectedTip) : '—'}</strong></div>
        <div><span>Head height</span><strong>{head ? number.format(head.height) : '—'}</strong></div>
        <div><span>Recent blocks</span><strong>{number.format(blocks.length)}</strong></div>
        <div><span>DAG tips</span><strong>{snapshot ? number.format(snapshot.status.tipCount) : '—'}</strong></div>
      </section>

      <section className="dag-live-layout">
        <article className="panel dag-frontier-panel">
          <div className="panel-header">
            <div><span className="eyebrow">Newest first</span><h3>Frontier timeline</h3></div>
            <span className={`status-badge ${snapshot ? 'success' : 'warning'}`}>{snapshot ? 'Live data' : 'No data'}</span>
          </div>
          <div className="dag-frontier-feed">
            {blocks.length === 0 && <div className="dag-empty">Start the node and wait for the local RPC snapshot.</div>}
            {blocks.map((block, index) => (
              <article className={`dag-frontier-block ${blockTone(block, selectedTip)}`} key={block.hash}>
                <div className="dag-frontier-index">{String(index + 1).padStart(2, '0')}</div>
                <div className="dag-node-glyph"><i /><span>{block.parentCount}</span></div>
                <div className="dag-frontier-copy">
                  <strong title={block.hash}>{shortHash(block.hash)}</strong>
                  <small>Height {number.format(block.height)} · Blue score {number.format(block.blueScore)}</small>
                </div>
                <div className="dag-frontier-meta">
                  <span>{number.format(block.txCount)} tx</span>
                  <time>{timestamp(block.timestamp)}</time>
                </div>
              </article>
            ))}
          </div>
        </article>

        <aside className="panel dag-inspector-panel">
          <div className="panel-header"><div><span className="eyebrow">Current head</span><h3>Block evidence</h3></div></div>
          {head ? (
            <div className="dag-inspector-content">
              <div className="selected-hash">{head.hash}</div>
              <div className="detail-list">
                <div><span>Height</span><strong>{number.format(head.height)}</strong></div>
                <div><span>Blue score</span><strong>{number.format(head.blueScore)}</strong></div>
                <div><span>Transactions</span><strong>{number.format(head.txCount)}</strong></div>
                <div><span>Parent count</span><strong>{number.format(head.parentCount)}</strong></div>
                <div><span>Timestamp</span><strong>{timestamp(head.timestamp)}</strong></div>
                <div><span>Selected tip</span><strong className={head.hash === selectedTip ? 'success-text' : 'warning-text'}>{head.hash === selectedTip ? 'Yes' : 'No'}</strong></div>
              </div>
            </div>
          ) : <p className="dag-empty">No recent block is available.</p>}
        </aside>
      </section>

      <article className="panel dag-table-panel lower-grid">
        <div className="panel-header"><div><span className="eyebrow">Exact response fields</span><h3>Recent blocks</h3></div></div>
        <div className="dag-table-scroll">
          <table>
            <thead><tr><th>Hash</th><th>Height</th><th>Blue score</th><th>Transactions</th><th>Parents</th><th>Timestamp</th></tr></thead>
            <tbody>
              {blocks.map((block) => (
                <tr key={block.hash} className={block.hash === selectedTip ? 'selected-row' : ''}>
                  <td><code title={block.hash}>{shortHash(block.hash)}</code></td>
                  <td>{number.format(block.height)}</td>
                  <td>{number.format(block.blueScore)}</td>
                  <td>{number.format(block.txCount)}</td>
                  <td>{number.format(block.parentCount)}</td>
                  <td>{timestamp(block.timestamp)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {blocks.length === 0 && <div className="dag-empty">No recent blocks were returned.</div>}
        </div>
      </article>
    </>
  )
}
