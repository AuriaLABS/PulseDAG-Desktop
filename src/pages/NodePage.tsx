import type { NodePreferences } from '../types'

type NodePageProps = {
  preferences: NodePreferences
  onOpenSettings: () => void
}

export function NodePage({ preferences, onOpenSettings }: NodePageProps) {
  return (
    <section className="page-grid">
      <article className="panel node-control-panel">
        <div className="panel-header">
          <div><span className="eyebrow">Process control</span><h3>PulseDAG node</h3></div>
          <span className="status-badge warning">Stopped</span>
        </div>
        <div className="node-control-body">
          <div className="node-avatar"><span>PD</span></div>
          <div>
            <strong>{preferences.executablePath || 'No executable selected'}</strong>
            <small>{preferences.rpcEndpoint}</small>
          </div>
        </div>
        <div className="button-row">
          <button className="primary-button" disabled>Start node</button>
          <button className="secondary-button" disabled>Restart</button>
          <button className="secondary-button" onClick={onOpenSettings}>Configure</button>
        </div>
      </article>

      <article className="panel">
        <div className="panel-header"><div><span className="eyebrow">Runtime</span><h3>Supervision plan</h3></div></div>
        <div className="detail-list">
          <div><span>Executable validation</span><strong>Next milestone</strong></div>
          <div><span>Process lifecycle</span><strong>Next milestone</strong></div>
          <div><span>Crash recovery</span><strong>Planned</strong></div>
          <div><span>Signed updates</span><strong>Planned</strong></div>
        </div>
      </article>
    </section>
  )
}
