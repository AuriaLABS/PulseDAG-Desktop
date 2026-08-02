import type { DiagnosticExportResult, LogEntry } from '../types'

type LogsPageProps = {
  entries: LogEntry[]
  running: boolean
  busy: boolean
  error: string
  exportResult: DiagnosticExportResult | null
  onClear: () => void
  onExport: () => void
}

export function LogsPage({ entries, running, busy, error, exportResult, onClear, onExport }: LogsPageProps) {
  return (
    <section className="panel logs-panel">
      <div className="panel-header">
        <div><span className="eyebrow">Captured locally</span><h3>pulsedagd output</h3></div>
        <div className="logs-actions">
          <span className={`status-badge ${running ? 'success' : 'warning'}`}>{running ? 'Streaming' : 'Idle'}</span>
          <button className="secondary-button compact-button" onClick={onExport} disabled={busy}>Export diagnostics…</button>
          <button className="secondary-button compact-button" onClick={onClear} disabled={busy}>Clear</button>
        </div>
      </div>
      {error && <div className="notice notice-error inline-notice">{error}</div>}
      {exportResult && (
        <div className="notice notice-success inline-notice">
          Exported {exportResult.logEntries} redacted log entries ({(exportResult.bytesWritten / 1024).toFixed(1)} KiB) to <code>{exportResult.path}</code>.
        </div>
      )}
      <div className="diagnostic-boundary">
        Diagnostic export includes runtime, loopback health, binary digest and captured logs. Executable, data and home-directory paths are replaced before writing the JSON file.
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
