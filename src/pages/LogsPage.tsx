import type { LogEntry } from '../types'

type LogsPageProps = {
  entries: LogEntry[]
  running: boolean
  onClear: () => void
}

export function LogsPage({ entries, running, onClear }: LogsPageProps) {
  return (
    <section className="panel logs-panel">
      <div className="panel-header">
        <div><span className="eyebrow">Captured locally</span><h3>pulsedagd output</h3></div>
        <div className="logs-actions"><span className={`status-badge ${running ? 'success' : 'warning'}`}>{running ? 'Streaming' : 'Idle'}</span><button className="secondary-button compact-button" onClick={onClear}>Clear</button></div>
      </div>
      <div className="log-console" role="log" aria-live="polite">
        {entries.length === 0 && <div className="log-empty">No process output has been captured yet.</div>}
        {entries.map((entry) => (
          <div className={`log-line stream-${entry.stream}`} key={entry.sequence}>
            <time>{new Date(entry.timestampMs).toLocaleTimeString()}</time>
            <span>{entry.stream}</span>
            <code>{entry.message}</code>
          </div>
        ))}
      </div>
    </section>
  )
}
