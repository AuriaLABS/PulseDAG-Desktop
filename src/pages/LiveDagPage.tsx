import { useCallback, useEffect, useRef, useState } from 'react'
import { getBlockDetail, getTransactionDetail, loadNodePreferences } from '../lib/desktop'
import type {
  BlockDetail,
  NodeObservability,
  RecentDagBlock,
  TransactionDetail,
} from '../types'

const number = new Intl.NumberFormat('en-US')

type LiveDagPageProps = {
  snapshot: NodeObservability | null
  loading: boolean
  error: string
  rpcReachable: boolean
  endpoint?: string
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

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function LiveDagPage({
  snapshot,
  loading,
  error,
  rpcReachable,
  endpoint = loadNodePreferences().rpcEndpoint,
  onRefresh,
}: LiveDagPageProps) {
  const blocks = snapshot?.blocks ?? []
  const selectedTip = snapshot?.status.selectedTip ?? null
  const head = blocks[0] ?? null
  const [selectedHash, setSelectedHash] = useState<string | null>(null)
  const [blockDetail, setBlockDetail] = useState<BlockDetail | null>(null)
  const [blockLoading, setBlockLoading] = useState(false)
  const [blockError, setBlockError] = useState('')
  const [selectedTxid, setSelectedTxid] = useState<string | null>(null)
  const [transactionDetail, setTransactionDetail] = useState<TransactionDetail | null>(null)
  const [transactionLoading, setTransactionLoading] = useState(false)
  const [transactionError, setTransactionError] = useState('')
  const blockRequest = useRef(0)
  const transactionRequest = useRef(0)
  const currentBlockHash = useRef<string | null>(null)
  const currentTxid = useRef<string | null>(null)

  const loadBlock = useCallback(async (hash: string) => {
    const request = ++blockRequest.current
    const preserveCurrent = currentBlockHash.current === hash
    currentBlockHash.current = hash
    setSelectedHash(hash)
    setBlockLoading(true)
    setBlockError('')
    setSelectedTxid(null)
    setTransactionDetail(null)
    setTransactionError('')
    currentTxid.current = null
    if (!preserveCurrent) setBlockDetail(null)

    try {
      const detail = await getBlockDetail(endpoint, hash)
      if (request === blockRequest.current) setBlockDetail(detail)
    } catch (loadError) {
      if (request === blockRequest.current) setBlockError(readableError(loadError))
    } finally {
      if (request === blockRequest.current) setBlockLoading(false)
    }
  }, [endpoint])

  const loadTransaction = useCallback(async (txid: string) => {
    const request = ++transactionRequest.current
    const preserveCurrent = currentTxid.current === txid
    currentTxid.current = txid
    setSelectedTxid(txid)
    setTransactionLoading(true)
    setTransactionError('')
    if (!preserveCurrent) setTransactionDetail(null)

    try {
      const detail = await getTransactionDetail(endpoint, txid)
      if (request === transactionRequest.current) setTransactionDetail(detail)
    } catch (loadError) {
      if (request === transactionRequest.current) setTransactionError(readableError(loadError))
    } finally {
      if (request === transactionRequest.current) setTransactionLoading(false)
    }
  }, [endpoint])

  useEffect(() => {
    blockRequest.current += 1
    transactionRequest.current += 1
    currentBlockHash.current = null
    currentTxid.current = null
    setSelectedHash(null)
    setBlockDetail(null)
    setBlockError('')
    setSelectedTxid(null)
    setTransactionDetail(null)
    setTransactionError('')
  }, [endpoint])

  useEffect(() => {
    if (!rpcReachable || !head || selectedHash) return
    void loadBlock(head.hash)
  }, [head, loadBlock, rpcReachable, selectedHash])

  const overview = blockDetail?.overview.hash === selectedHash ? blockDetail.overview : null
  const transactions = blockDetail?.overview.hash === selectedHash ? blockDetail.transactions : null
  const transaction = transactionDetail?.transaction.txid === selectedTxid
    ? transactionDetail.transaction
    : null

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
          <p>Select a block to load its real parent and child hashes, confirmations and up to 100 confirmed transactions. Transaction drill-down exposes only the lookup fields returned by the local node.</p>
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
              <button
                type="button"
                className={`dag-frontier-block ${blockTone(block, selectedTip)} ${selectedHash === block.hash ? 'active' : ''}`}
                key={block.hash}
                onClick={() => void loadBlock(block.hash)}
                aria-pressed={selectedHash === block.hash}
              >
                <span className="dag-frontier-index">{String(index + 1).padStart(2, '0')}</span>
                <span className="dag-node-glyph"><i /><span>{block.parentCount}</span></span>
                <span className="dag-frontier-copy">
                  <strong title={block.hash}>{shortHash(block.hash)}</strong>
                  <small>Height {number.format(block.height)} · Blue score {number.format(block.blueScore)}</small>
                </span>
                <span className="dag-frontier-meta">
                  <span>{number.format(block.txCount)} tx</span>
                  <time>{timestamp(block.timestamp)}</time>
                </span>
              </button>
            ))}
          </div>
        </article>

        <aside className="panel dag-inspector-panel">
          <div className="panel-header">
            <div><span className="eyebrow">Bounded entity lookup</span><h3>Block detail</h3></div>
            <button
              className="compact-button secondary-button"
              onClick={() => selectedHash && void loadBlock(selectedHash)}
              disabled={!selectedHash || blockLoading || !rpcReachable}
            >
              {blockLoading ? 'Loading…' : 'Refresh'}
            </button>
          </div>
          {blockError && <div className="notice notice-error inline-notice" role="alert">{blockError}</div>}
          {overview ? (
            <div className="dag-inspector-content">
              <div className="selected-hash">{overview.hash}</div>
              <div className="detail-list">
                <div><span>Height</span><strong>{number.format(overview.height)}</strong></div>
                <div><span>Blue score</span><strong>{number.format(overview.blueScore)}</strong></div>
                <div><span>Confirmations</span><strong>{number.format(overview.confirmations)}</strong></div>
                <div><span>Transactions</span><strong>{number.format(overview.txCount)}</strong></div>
                <div><span>Timestamp</span><strong>{timestamp(overview.timestamp)}</strong></div>
                <div><span>DAG tip</span><strong className={overview.isTip ? 'success-text' : ''}>{overview.isTip ? 'Yes' : 'No'}</strong></div>
                <div><span>Selected tip</span><strong className={overview.hash === overview.selectedTip ? 'success-text' : 'warning-text'}>{overview.hash === overview.selectedTip ? 'Yes' : 'No'}</strong></div>
                <div><span>Lookup latency</span><strong>{number.format(blockDetail?.latencyMs ?? 0)} ms</strong></div>
              </div>
              <div className="entity-relations">
                <span className="eyebrow">Parents</span>
                <div className="hash-chip-list">
                  {overview.parentHashes.length === 0 && <small>Genesis boundary</small>}
                  {overview.parentHashes.map((hash) => (
                    <button type="button" className="hash-chip" key={hash} title={hash} onClick={() => void loadBlock(hash)}>
                      {shortHash(hash)}
                    </button>
                  ))}
                </div>
              </div>
              <div className="entity-relations">
                <span className="eyebrow">Children</span>
                <div className="hash-chip-list">
                  {overview.childHashes.length === 0 && <small>No children returned</small>}
                  {overview.childHashes.map((hash) => (
                    <button type="button" className="hash-chip" key={hash} title={hash} onClick={() => void loadBlock(hash)}>
                      {shortHash(hash)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <p className="dag-empty">{blockLoading ? 'Loading the selected block…' : 'Select a recent block to inspect it.'}</p>
          )}
        </aside>
      </section>

      <article className="panel entity-transactions-panel lower-grid">
        <div className="panel-header">
          <div><span className="eyebrow">Confirmed context only</span><h3>Block transactions</h3></div>
          {transactions && <span className="status-badge success">{number.format(transactions.count)} of {number.format(transactions.total)}</span>}
        </div>
        {transactions?.hasMore && (
          <div className="notice notice-warning inline-notice">This bounded view shows the first {number.format(transactions.limit)} transactions returned by the node.</div>
        )}
        <div className="dag-table-scroll entity-table-scroll">
          <table>
            <thead><tr><th>Transaction</th><th>Fee</th><th>Inputs</th><th>Outputs</th><th>Context</th></tr></thead>
            <tbody>
              {transactions?.transactions.map((item) => (
                <tr key={item.txid} className={selectedTxid === item.txid ? 'selected-row' : ''}>
                  <td>
                    <button type="button" className="entity-link" title={item.txid} onClick={() => void loadTransaction(item.txid)}>
                      {shortHash(item.txid)}
                    </button>
                  </td>
                  <td>{number.format(item.fee)}</td>
                  <td>{number.format(item.inputs)}</td>
                  <td>{number.format(item.outputs)}</td>
                  <td>{item.context}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {!transactions && <div className="dag-empty">{blockLoading ? 'Loading transactions…' : 'Select a block to load its confirmed transactions.'}</div>}
          {transactions && transactions.transactions.length === 0 && <div className="dag-empty">This block contains no transactions.</div>}
        </div>
      </article>

      {selectedTxid && (
        <article className="panel transaction-detail-panel lower-grid">
          <div className="panel-header">
            <div><span className="eyebrow">Exact transaction lookup</span><h3>Transaction detail</h3></div>
            <div className="entity-header-actions">
              <button className="compact-button secondary-button" onClick={() => void loadTransaction(selectedTxid)} disabled={transactionLoading || !rpcReachable}>
                {transactionLoading ? 'Loading…' : 'Refresh'}
              </button>
              <button className="compact-button secondary-button" onClick={() => {
                transactionRequest.current += 1
                currentTxid.current = null
                setSelectedTxid(null)
                setTransactionDetail(null)
                setTransactionError('')
              }}>Close</button>
            </div>
          </div>
          {transactionError && <div className="notice notice-error inline-notice" role="alert">{transactionError}</div>}
          {transaction ? (
            <div className="transaction-detail-grid">
              <section>
                <div className="selected-hash">{transaction.txid}</div>
                <div className="detail-list">
                  <div><span>Status</span><strong className={transaction.isConfirmed ? 'success-text' : 'warning-text'}>{transaction.status}</strong></div>
                  <div><span>Fee</span><strong>{number.format(transaction.fee)}</strong></div>
                  <div><span>Nonce</span><strong>{number.format(transaction.nonce)}</strong></div>
                  <div><span>Confirmations</span><strong>{transaction.confirmations === null ? '—' : number.format(transaction.confirmations)}</strong></div>
                  <div><span>Inputs</span><strong>{number.format(transaction.inputs.length)}</strong></div>
                  <div><span>Outputs</span><strong>{number.format(transaction.outputs.length)}</strong></div>
                  <div><span>Lookup latency</span><strong>{number.format(transactionDetail?.latencyMs ?? 0)} ms</strong></div>
                </div>
                {transaction.blockHash && (
                  <button type="button" className="block-context-button" onClick={() => void loadBlock(transaction.blockHash!)}>
                    Open block {transaction.blockHeight === null ? '' : `#${number.format(transaction.blockHeight)}`}
                    <code>{shortHash(transaction.blockHash)}</code>
                  </button>
                )}
              </section>
              <section className="transaction-io-section">
                <div><span className="eyebrow">Inputs</span><h4>Previous outpoints</h4></div>
                <div className="transaction-io-list">
                  {transaction.inputs.length === 0 && <p className="dag-empty">No previous outpoints.</p>}
                  {transaction.inputs.map((input, index) => (
                    <div className="transaction-io-row" key={`${input.txid}:${input.index}:${index}`}>
                      <span>#{number.format(input.index)}</span>
                      <code title={input.txid}>{shortHash(input.txid)}</code>
                    </div>
                  ))}
                </div>
              </section>
              <section className="transaction-io-section">
                <div><span className="eyebrow">Outputs</span><h4>Addresses and raw units</h4></div>
                <div className="transaction-io-list">
                  {transaction.outputs.length === 0 && <p className="dag-empty">No outputs.</p>}
                  {transaction.outputs.map((output, index) => (
                    <div className="transaction-output-row" key={`${output.address}:${index}`}>
                      <code title={output.address}>{output.address}</code>
                      <strong>{number.format(output.amount)}</strong>
                    </div>
                  ))}
                </div>
              </section>
            </div>
          ) : <p className="dag-empty">{transactionLoading ? 'Loading transaction evidence…' : 'Transaction detail is unavailable.'}</p>}
        </article>
      )}

      <article className="panel dag-table-panel lower-grid">
        <div className="panel-header"><div><span className="eyebrow">Exact response fields</span><h3>Recent blocks</h3></div></div>
        <div className="dag-table-scroll">
          <table>
            <thead><tr><th>Hash</th><th>Height</th><th>Blue score</th><th>Transactions</th><th>Parents</th><th>Timestamp</th></tr></thead>
            <tbody>
              {blocks.map((block) => (
                <tr key={block.hash} className={selectedHash === block.hash ? 'selected-row' : block.hash === selectedTip ? 'selected-row' : ''}>
                  <td>
                    <button type="button" className="entity-link" title={block.hash} onClick={() => void loadBlock(block.hash)}>
                      {shortHash(block.hash)}
                    </button>
                  </td>
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
