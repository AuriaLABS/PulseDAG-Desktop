import { useState, type FormEvent } from 'react'
import type { NodePreferences } from '../types'

type SettingsPageProps = {
  value: NodePreferences
  onSave: (preferences: NodePreferences) => void
}

export function SettingsPage({ value, onSave }: SettingsPageProps) {
  const [draft, setDraft] = useState(value)
  const [saved, setSaved] = useState(false)

  function update<K extends keyof NodePreferences>(key: K, nextValue: NodePreferences[K]) {
    setDraft((current) => ({ ...current, [key]: nextValue }))
    setSaved(false)
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    onSave(draft)
    setSaved(true)
  }

  return (
    <form className="settings-layout" onSubmit={submit}>
      <section className="panel settings-panel">
        <div className="panel-header"><div><span className="eyebrow">Local configuration</span><h3>Node executable</h3></div></div>
        <label>
          <span>pulsedagd path</span>
          <input value={draft.executablePath} onChange={(event) => update('executablePath', event.target.value)} placeholder="C:\\PulseDAG\\pulsedagd.exe or /usr/local/bin/pulsedagd" />
          <small>A native file picker and binary verification will replace this text field.</small>
        </label>
        <label>
          <span>RPC endpoint</span>
          <input value={draft.rpcEndpoint} onChange={(event) => update('rpcEndpoint', event.target.value)} placeholder="http://127.0.0.1:8080/api/v1" />
          <small>Keep administrative and wallet RPC bound to loopback.</small>
        </label>
        <label>
          <span>Network</span>
          <select value={draft.network} onChange={(event) => update('network', event.target.value as NodePreferences['network'])}>
            <option value="private-testnet">Private testnet</option>
            <option value="devnet">Devnet</option>
            <option value="testnet">Testnet</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
        <label className="switch-row">
          <span><strong>Launch on startup</strong><small>Available after process supervision is implemented.</small></span>
          <input type="checkbox" checked={draft.launchOnStartup} onChange={(event) => update('launchOnStartup', event.target.checked)} />
        </label>
        <div className="button-row">
          <button className="primary-button" type="submit">Save local settings</button>
          {saved && <span className="saved-message">Saved</span>}
        </div>
      </section>

      <aside className="panel settings-aside">
        <span className="eyebrow">Milestone boundary</span>
        <h3>Configuration only</h3>
        <p>This scaffold stores non-sensitive preferences locally. Node execution, credential storage and RPC validation will be implemented in Rust before controls are enabled.</p>
      </aside>
    </form>
  )
}
