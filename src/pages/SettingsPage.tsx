import { useEffect, useState, type ChangeEvent, type FormEvent } from 'react'
import type { BinaryInfo, BinaryProvenance, NodePreferences, ReleaseVerification } from '../types'

const V24_FINAL_SOURCE = '876b48826a3875b729888edb88e2b0eea15bb717'

type SettingsPageProps = {
  value: NodePreferences
  binaryInfo: BinaryInfo | null
  releaseVerification: ReleaseVerification | null
  binaryProvenance: BinaryProvenance | null
  busy: boolean
  error: string
  onSave: (preferences: NodePreferences) => void
  onDetect: () => Promise<BinaryInfo | null>
  onValidate: (path: string) => Promise<BinaryInfo | null>
  onPickExecutable: () => Promise<string | null>
  onPickDataDirectory: () => Promise<string | null>
  onVerifyRelease: (profile: NodePreferences['configProfile']) => Promise<ReleaseVerification | null>
  onBindProvenance: (
    path: string,
    profile: NodePreferences['configProfile'],
  ) => Promise<BinaryProvenance | null>
}

export function SettingsPage({
  value,
  binaryInfo,
  releaseVerification,
  binaryProvenance,
  busy,
  error,
  onSave,
  onDetect,
  onValidate,
  onPickExecutable,
  onPickDataDirectory,
  onVerifyRelease,
  onBindProvenance,
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
      await onVerifyRelease(draft.configProfile)
    } finally {
      setChecking(false)
    }
  }

  async function bindProvenance() {
    setChecking(true)
    try {
      await onBindProvenance(draft.executablePath, draft.configProfile)
    } finally {
      setChecking(false)
    }
  }

  const privateMode = draft.configProfile === 'private'
  const privateHasFinalProvenance = binaryProvenance?.approved
    && binaryProvenance.releaseTag === 'v2.4.0'
    && binaryProvenance.sourceCommit === V24_FINAL_SOURCE
    && binaryProvenance.archiveName.startsWith('pulsedagd-v2.4.0-')
  const privateNeedsProvenance = privateMode && !privateHasFinalProvenance
  const releaseMatchesProfile = privateMode
    ? releaseVerification?.approved
      && releaseVerification.releaseTag === 'v2.4.0'
      && releaseVerification.sourceCommit === V24_FINAL_SOURCE
    : releaseVerification?.approved && releaseVerification.releaseTag === 'v2.3.0'

  return (
    <form className="settings-layout" onSubmit={submit}>
      <section className="panel settings-panel">
        <div className="panel-header"><div><span className="eyebrow">Local configuration</span><h3>Node runtime</h3></div></div>
        {error && <div className="notice notice-error inline-notice">{error}</div>}
        {privateNeedsProvenance && (
          <div className="notice notice-warning inline-notice">
            Private v2.4.0 requires pulsedagd from the final Task31 release, linked byte-for-byte in this desktop session. A v2.3 or candidate proof is not accepted.
          </div>
        )}
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
          <small>{privateMode
            ? 'Private v2.4.0 requires a new empty directory on first use. Desktop writes identity + final-source markers before RocksDB and never relabels or deletes older state.'
            : 'RocksDB and the persistent P2P identity are kept below this directory.'}</small>
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
            <option value="private">Private v2.4.0 · final Task31 release required</option>
          </select>
          <small>{privateMode
            ? 'Private is forced to the v2.4 identity in isolated single-node mode: loopback RPC, P2P off, public-testnet clock off and contracts off.'
            : 'The desktop always overrides RPC to the selected loopback origin and disables administrative endpoints.'}</small>
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
          <span className="eyebrow">{privateMode ? 'Final v2.4.0 release evidence' : 'Approved v2.3.0 release evidence'}</span>
          <h3>Verify and link the node</h3>
          <p>{privateMode
            ? 'Verify the final v2.4.0 Task31 archive against the frozen release asset set, GitHub digest, sidecar, manifest, consolidated checksums and native-smoke provenance. Then compare the selected pulsedagd byte-for-byte with the archive member.'
            : 'Verify the original v2.3.0 ZIP or TAR.GZ against GitHub. Then Rust reads pulsedagd directly from that archive and compares its bytes with the selected executable without extracting or running anything.'}</p>
          <button className="secondary-button full-width-button" type="button" onClick={() => void verifyRelease()} disabled={busy || checking}>Verify {privateMode ? 'final v2.4.0' : 'v2.3.0'} release archive…</button>
          {releaseVerification && (
            <div className={`release-result ${releaseVerification.approved ? 'approved' : 'rejected'}`}>
              <strong>{releaseMatchesProfile ? 'Approved archive for selected profile' : releaseVerification.approved ? 'Verified for a different profile' : 'Digest or provenance mismatch'}</strong>
              <span>{releaseVerification.archiveName}</span>
              <code>{releaseVerification.sha256}</code>
              <small>{releaseVerification.message}</small>
            </div>
          )}
          <button
            className="primary-button full-width-button"
            type="button"
            onClick={() => void bindProvenance()}
            disabled={busy || checking || !releaseMatchesProfile || !draft.executablePath}
          >
            Link executable to {privateMode ? 'final v2.4.0' : 'approved v2.3.0'} archive
          </button>
          {binaryProvenance && (
            <div className={`provenance-result ${privateMode && !privateHasFinalProvenance ? 'rejected' : 'approved'}`}>
              <div className="provenance-result-header">
                <strong>{privateMode && !privateHasFinalProvenance ? 'Proof not valid for private v2.4.0' : 'Byte-for-byte match'}</strong>
                <span>{binaryProvenance.target}</span>
              </div>
              <small>{binaryProvenance.archiveName}</small>
              <code>{binaryProvenance.embeddedBinarySha256}</code>
              <div className="detail-list compact-details">
                <div><span>Release</span><strong>{binaryProvenance.releaseTag}</strong></div>
                <div><span>Source</span><strong>{binaryProvenance.sourceCommit.slice(0, 12)}</strong></div>
                <div><span>Embedded file</span><strong>{binaryProvenance.embeddedPath}</strong></div>
                <div><span>Size</span><strong>{(binaryProvenance.embeddedBinarySizeBytes / 1_048_576).toFixed(2)} MiB</strong></div>
              </div>
              <p>{binaryProvenance.message}</p>
            </div>
          )}
          <div className="notice notice-warning release-boundary">The proof is held in native memory for this desktop session. The binary is hashed again before every launch; changing it invalidates the proof.</div>
        </section>

        <section className="panel settings-aside">
          <span className="eyebrow">Security boundary</span>
          <h3>Explicit launch environment</h3>
          <p>{privateMode
            ? 'Private v2.4.0 clears inherited PULSEDAG_* values and supplies the frozen chain identity plus an isolated single-node safety contract before starting pulsedagd.'
            : 'Before starting pulsedagd, the Rust backend removes inherited PULSEDAG_* variables and supplies only the selected profile, loopback RPC binding, state paths and administrative-disable flags.'}</p>
          <div className="detail-list compact-details">
            <div><span>Private profile</span><strong>{privateMode ? 'Final v2.4 proof only' : 'Proof required when selected'}</strong></div>
            <div><span>Admin RPC</span><strong>Forced off</strong></div>
            <div><span>RPC exposure</span><strong>Loopback only</strong></div>
            <div><span>P2P in private v2.4</span><strong>Forced off</strong></div>
            <div><span>Public-testnet clock</span><strong>Forced off</strong></div>
            <div><span>Contracts</span><strong>Forced off</strong></div>
            <div><span>Secrets</span><strong>Not accepted</strong></div>
            <div><span>Stop policy</span><strong>TERM, then kill</strong></div>
          </div>
        </section>
      </aside>
    </form>
  )
}