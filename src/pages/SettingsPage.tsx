import { useEffect, useState, type ChangeEvent, type FormEvent } from 'react'
import type { BinaryInfo, NodePreferences, ReleaseVerification } from '../types'

type SettingsPageProps = {
  value: NodePreferences
  binaryInfo: BinaryInfo | null
  releaseVerification: ReleaseVerification | null
  busy: boolean
  error: string
  onSave: (preferences: NodePreferences) => void
  onDetect: () => Promise<BinaryInfo | null>
  onValidate: (path: string) => Promise<BinaryInfo | null>
  onPickExecutable: () => Promise<string | null>
  onPickDataDirectory: () => Promise<string | null>
  onVerifyRelease: () => Promise<ReleaseVerification | null>
}

export function SettingsPage({
  value,
  binaryInfo,
  releaseVerification,
  busy,
  error,
  onSave,
  onDetect,
  onValidate,
  onPickExecutable,
  onPickDataDirectory,
  onVerifyRelease,
}: SettingsPageProps) {
  const [draft, setDraft] = useState(value)
  const [saved, setSaved] = useState(false)
  const [checking, setChecking] = useState(false)

  useEffect(() => setDraft(value), [value])

  function update<K extends keyof NodePreferences>(key: K, nextValue: NodePreferences[K]) {
    setDraft((current) => ({ ...current, [key]: nextValue }))
    setSaved(false)
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    onSave(draft)
    setSaved(true)
  }

  async function detect() {
    setChecking(true)
    try {
      const info = await onDetect()
      if (info) setDraft((current) => ({ ...current, executablePath: info.path }))
    } finally {
      setChecking(false)
    }
  }

  async function pickExecutable() {
    setChecking(true)
    try {
      const path = await onPickExecutable()
      if (!path) return
      setDraft((current) => ({ ...current, executablePath: path }))
      await onValidate(path)
    } finally {
      setChecking(false)
    }
  }

  async function pickDataDirectory() {
    setChecking(true)
    try {
      const path = await onPickDataDirectory()
      if (path) update('dataDirectory', path)
    } finally {
      setChecking(false)
    }
  }

  async function validate() {
    setChecking(true)
    try {
      await onValidate(draft.executablePath)
    } finally {
      setChecking(false)
    }
  }

  async function verifyRelease() {
    setChecking(true)
    try {
      await onVerifyRelease()
    } finally {
      setChecking(false)
    }
  }

  return (
    <form className="settings-layout" onSubmit={submit}>
      <section className="panel settings-panel">
        <div className="panel-header"><div><span className="eyebrow">Local configuration</span><h3>Node runtime</h3></div></div>
        {error && <div className="notice notice-error inline-notice">{error}</div>}
        <label>
          <span>pulsedagd path</span>
          <div className="input-action-row">
            <input value={draft.executablePath} onChange={(event: ChangeEvent<HTMLInputElement>) => update('executablePath', event.target.value)} placeholder="C:\\PulseDAG\\pulsedagd.exe or /usr/local/bin/pulsedagd" />
            <button className="secondary-button compact-button" type="button" onClick={() => void pickExecutable()} disabled={busy || checking}>Browse…</button>
          </div>
          <small>The native backend requires the exact file name and computes SHA-256 before launch.</small>
        </label>
        <div className="button-row field-actions">
          <button className="secondary-button" type="button" onClick={() => void detect()} disabled={busy || checking}>{checking ? 'Checking…' : 'Detect automatically'}</button>
          <button className="secondary-button" type="button" onClick={() => void validate()} disabled={busy || checking || !draft.executablePath}>Validate selected binary</button>
        </div>
        {binaryInfo && (
          <div className="validation-card">
            <strong>{binaryInfo.fileName}</strong>
            <span>{(binaryInfo.sizeBytes / 1_048_576).toFixed(2)} MiB</span>
            <code>{binaryInfo.sha256}</code>
          </div>
        )}
        <label>
          <span>Persistent data directory</span>
          <div className="input-action-row">
            <input value={draft.dataDirectory} onChange={(event: ChangeEvent<HTMLInputElement>) => update('dataDirectory', event.target.value)} placeholder="C:\\PulseDAG\\data or /home/user/.pulsedag" />
            <button className="secondary-button compact-button" type="button" onClick={() => void pickDataDirectory()} disabled={busy || checking}>Browse…</button>
          </div>
          <small>RocksDB and the persistent P2P identity are kept below this directory.</small>
        </label>
        <label>
          <span>RPC origin</span>
          <input value={draft.rpcEndpoint} onChange={(event: ChangeEvent<HTMLInputElement>) => update('rpcEndpoint', event.target.value)} placeholder="http://127.0.0.1:8080" />
          <small>Only HTTP on localhost, 127.0.0.1 or ::1 is accepted. Do not add /api/v1.</small>
        </label>
        <label>
          <span>Configuration profile</span>
          <select value={draft.configProfile} onChange={(event: ChangeEvent<HTMLSelectElement>) => update('configProfile', event.target.value as NodePreferences['configProfile'])}>
            <option value="dev">Development · isolated</option>
            <option value="local">Local network · P2P enabled</option>
            <option value="private">Private testnet · operator profile</option>
          </select>
          <small>The desktop always overrides RPC to the selected loopback origin and disables administrative endpoints.</small>
        </label>
        <label className="switch-row disabled-setting">
          <span><strong>Launch on startup</strong><small>Reserved for a later milestone after crash-recovery policy is defined.</small></span>
          <input type="checkbox" checked={draft.launchOnStartup} onChange={(event: ChangeEvent<HTMLInputElement>) => update('launchOnStartup', event.target.checked)} disabled />
        </label>
        <div className="button-row">
          <button className="primary-button" type="submit" disabled={busy}>Save local settings</button>
          {saved && <span className="saved-message">Saved</span>}
        </div>
      </section>

      <aside className="settings-side-stack">
        <section className="panel settings-aside">
          <span className="eyebrow">Approved release evidence</span>
          <h3>Verify the downloaded archive</h3>
          <p>Select the original v2.3.0 ZIP or TAR.GZ. Rust hashes it locally and compares it with the digest published by GitHub for the approved PulseDAG release and source commit.</p>
          <button className="secondary-button full-width-button" type="button" onClick={() => void verifyRelease()} disabled={busy || checking}>Verify release archive…</button>
          {releaseVerification && (
            <div className={`release-result ${releaseVerification.approved ? 'approved' : 'rejected'}`}>
              <strong>{releaseVerification.approved ? 'Approved archive' : 'Digest mismatch'}</strong>
              <span>{releaseVerification.archiveName}</span>
              <code>{releaseVerification.sha256}</code>
              <small>{releaseVerification.message}</small>
            </div>
          )}
          <div className="notice notice-warning release-boundary">This confirms the downloaded archive, not an extracted executable copied from an unknown source.</div>
        </section>

        <section className="panel settings-aside">
          <span className="eyebrow">Security boundary</span>
          <h3>Explicit launch environment</h3>
          <p>Before starting pulsedagd, the Rust backend removes inherited PULSEDAG_* variables and supplies only the selected profile, loopback RPC binding, state paths and administrative-disable flags.</p>
          <div className="detail-list compact-details">
            <div><span>Admin RPC</span><strong>Forced off</strong></div>
            <div><span>RPC exposure</span><strong>Loopback only</strong></div>
            <div><span>Secrets</span><strong>Not accepted</strong></div>
            <div><span>Stop policy</span><strong>TERM, then kill</strong></div>
          </div>
        </section>
      </aside>
    </form>
  )
}
