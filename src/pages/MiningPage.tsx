import { useEffect, useState, type ChangeEvent, type FormEvent } from 'react'
import {
  bindMinerBinaryToVerifiedArchive,
  getMinerBinaryProvenance,
  selectMinerReleaseArchive,
  verifyApprovedMinerReleaseArchive,
} from '../lib/desktop'
import type {
  BinaryInfo,
  BinaryProvenance,
  LogEntry,
  MinerRuntimeStatus,
  NodePreferences,
  NodeRuntimeStatus,
  ReleaseVerification,
  RpcHealth,
} from '../types'

type MiningPageProps = {
  preferences: NodePreferences
  minerBinaryInfo: BinaryInfo | null
  minerStatus: MinerRuntimeStatus
  nodeStatus: NodeRuntimeStatus
  rpcHealth: RpcHealth
  logs: LogEntry[]
  busy: boolean
  error: string
  onSave: (preferences: NodePreferences) => void
  onDetect: () => Promise<BinaryInfo | null>
  onValidate: (path: string) => Promise<BinaryInfo | null>
  onPickExecutable: () => Promise<string | null>
  onStart: () => void
  onStop: () => void
  onClearLogs: () => void
}

function numericValue(event: ChangeEvent<HTMLInputElement>, fallback: number): number {
  const value = Number(event.target.value)
  return Number.isFinite(value) ? Math.trunc(value) : fallback
}

function formatHashrate(value: number): string {
  if (value >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(2)} GH/s`
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)} MH/s`
  if (value >= 1_000) return `${(value / 1_000).toFixed(2)} kH/s`
  return `${value.toFixed(2)} H/s`
}

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function MiningPage({
  preferences,
  minerBinaryInfo,
  minerStatus,
  nodeStatus,
  rpcHealth,
  logs,
  busy,
  error,
  onSave,
  onDetect,
  onValidate,
  onPickExecutable,
  onStart,
  onStop,
  onClearLogs,
}: MiningPageProps) {
  const [draft, setDraft] = useState(preferences)
  const [checking, setChecking] = useState(false)
  const [saved, setSaved] = useState(false)
  const [releaseVerification, setReleaseVerification] = useState<ReleaseVerification | null>(null)
  const [minerProvenance, setMinerProvenance] = useState<BinaryProvenance | null>(null)
  const [provenanceError, setProvenanceError] = useState('')

  useEffect(() => setDraft(preferences), [preferences])
  useEffect(() => {
    void getMinerBinaryProvenance().then(setMinerProvenance)
  }, [])

  function update<K extends keyof NodePreferences>(key: K, value: NodePreferences[K]) {
    setDraft((current) => ({ ...current, [key]: value }))
    if (key === 'minerExecutablePath') {
      setMinerProvenance(null)
      setReleaseVerification(null)
    }
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
      if (info) {
        setDraft((current) => ({ ...current, minerExecutablePath: info.path }))
        setMinerProvenance(null)
        setReleaseVerification(null)
      }
    } finally {
      setChecking(false)
    }
  }

  async function browse() {
    setChecking(true)
    try {
      const path = await onPickExecutable()
      if (!path) return
      setDraft((current) => ({ ...current, minerExecutablePath: path }))
      setMinerProvenance(null)
      setReleaseVerification(null)
      await onValidate(path)
    } finally {
      setChecking(false)
    }
  }

  async function validate() {
    setChecking(true)
    try {
      await onValidate(draft.minerExecutablePath)
      setMinerProvenance(await getMinerBinaryProvenance())
    } finally {
      setChecking(false)
    }
  }

  async function verifyRelease() {
    setChecking(true)
    setProvenanceError('')
    try {
      const path = await selectMinerReleaseArchive()
      if (!path) return
      const verification = await verifyApprovedMinerReleaseArchive(path)
      setReleaseVerification(verification)
      setMinerProvenance(null)
      if (!verification.approved) setProvenanceError(verification.message)
    } catch (nextError) {
      setReleaseVerification(null)
      setMinerProvenance(null)
      setProvenanceError(readableError(nextError))
    } finally {
      setChecking(false)
    }
  }

  async function bindProvenance() {
    if (!releaseVerification?.approved || !draft.minerExecutablePath) return
    setChecking(true)
    setProvenanceError('')
    try {
      const proof = await bindMinerBinaryToVerifiedArchive(
        releaseVerification.archivePath,
        draft.minerExecutablePath,
      )
      setMinerProvenance(proof.approved ? proof : null)
      if (!proof.approved) setProvenanceError(proof.message)
    } catch (nextError) {
      setMinerProvenance(null)
      setProvenanceError(readableError(nextError))
    } finally {
      setChecking(false)
    }
  }

  const configured = draft.minerExecutablePath.trim().length > 0
    && draft.minerAddress.trim().length > 0
  const nodeReady = nodeStatus.running && rpcHealth.reachable
  const privateBlocked = draft.configProfile === 'private' && !minerProvenance?.approved
  const telemetry = minerStatus.telemetry

  return (
    <>
      {(error || provenanceError) && <div className="notice notice-error">{provenanceError || error}</div>}
      {privateBlocked && (
        <div className="notice notice-warning">
          Private-profile mining requires the selected pulsedag-miner binary to be linked byte-for-byte to its approved v2.3.0 release archive in this desktop session.
        </div>
      )}
      {!nodeReady && (
        <div className="notice notice-warning">
          Start the local node and wait for its loopback RPC before starting the miner.
        </div>
      )}

      <section className="mining-layout">
        <form className="panel mining-control-panel" onSubmit={submit}>
          <div className="panel-header">
            <div><span className="eyebrow">External PoW worker</span><h3>Standalone miner</h3></div>
            <span className={`status-badge ${minerStatus.running ? 'success' : 'warning'}`}>
              {minerStatus.running ? 'Mining' : 'Stopped'}
            </span>
          </div>

          <label>
            <span>pulsedag-miner path</span>
            <div className="input-action-row">
              <input
                value={draft.minerExecutablePath}
                onChange={(event) => update('minerExecutablePath', event.target.value)}
                placeholder="C:\\PulseDAG\\pulsedag-miner.exe or /usr/local/bin/pulsedag-miner"
                disabled={minerStatus.running}
              />
              <button className="secondary-button compact-button" type="button" onClick={() => void browse()} disabled={busy || checking || minerStatus.running}>Browse…</button>
            </div>
            <small>The native backend accepts only pulsedag-miner or pulsedag-miner.exe and hashes it before launch.</small>
          </label>

          <div className="button-row field-actions">
            <button className="secondary-button" type="button" onClick={() => void detect()} disabled={busy || checking || minerStatus.running}>{checking ? 'Checking…' : 'Detect automatically'}</button>
            <button className="secondary-button" type="button" onClick={() => void validate()} disabled={busy || checking || minerStatus.running || !draft.minerExecutablePath}>Validate miner</button>
          </div>

          {minerBinaryInfo && (
            <div className="validation-card">
              <strong>{minerBinaryInfo.fileName}</strong>
              <span>{(minerBinaryInfo.sizeBytes / 1_048_576).toFixed(2)} MiB</span>
              <code>{minerBinaryInfo.sha256}</code>
            </div>
          )}

          <section className="miner-provenance-card">
            <div>
              <span className="eyebrow">Approved miner evidence</span>
              <strong>{minerProvenance?.approved ? 'Executable linked' : 'Not linked in this session'}</strong>
              <small>Required only for the private profile. Development and local binaries remain available without release proof.</small>
            </div>
            <div className="button-row">
              <button className="secondary-button" type="button" onClick={() => void verifyRelease()} disabled={busy || checking || minerStatus.running}>Verify miner archive…</button>
              <button className="primary-button" type="button" onClick={() => void bindProvenance()} disabled={busy || checking || minerStatus.running || !releaseVerification?.approved || !draft.minerExecutablePath}>Link miner binary</button>
            </div>
            {releaseVerification && (
              <div className={`release-result ${releaseVerification.approved ? 'approved' : 'rejected'}`}>
                <strong>{releaseVerification.approved ? 'Approved miner archive' : 'Digest mismatch'}</strong>
                <span>{releaseVerification.archiveName}</span>
                <code>{releaseVerification.sha256}</code>
                <small>{releaseVerification.message}</small>
              </div>
            )}
            {minerProvenance && (
              <div className="provenance-result approved">
                <div className="provenance-result-header">
                  <strong>Byte-for-byte match</strong>
                  <span>{minerProvenance.target}</span>
                </div>
                <small>{minerProvenance.archiveName}</small>
                <code>{minerProvenance.embeddedBinarySha256}</code>
                <p>{minerProvenance.message}</p>
              </div>
            )}
          </section>

          <label>
            <span>Reward address</span>
            <input
              value={draft.minerAddress}
              onChange={(event) => update('minerAddress', event.target.value)}
              placeholder="PulseDAG address returned by your wallet tooling"
              disabled={minerStatus.running}
            />
            <small>Only the public reward address is passed to the official miner. No private key or seed is accepted.</small>
          </label>

          <div className="mining-field-grid">
            <label>
              <span>CPU threads</span>
              <input type="number" min={1} max={256} value={draft.minerThreads} onChange={(event) => update('minerThreads', numericValue(event, 1))} disabled={minerStatus.running} />
            </label>
            <label>
              <span>Maximum tries / template</span>
              <input type="number" min={1} max={100000000} value={draft.minerMaxTries} onChange={(event) => update('minerMaxTries', numericValue(event, 1))} disabled={minerStatus.running} />
            </label>
            <label>
              <span>Loop sleep (ms)</span>
              <input type="number" min={100} max={60000} value={draft.minerSleepMs} onChange={(event) => update('minerSleepMs', numericValue(event, 100))} disabled={minerStatus.running} />
            </label>
            <label>
              <span>Refresh before expiry (ms)</span>
              <input type="number" min={0} max={60000} value={draft.minerRefreshBeforeExpiryMs} onChange={(event) => update('minerRefreshBeforeExpiryMs', numericValue(event, 0))} disabled={minerStatus.running} />
            </label>
          </div>

          <label>
            <span>Worker ID</span>
            <input value={draft.minerWorkerId} onChange={(event) => update('minerWorkerId', event.target.value)} placeholder="desktop-worker" disabled={minerStatus.running} />
          </label>

          <label className="switch-row">
            <span><strong>Worker heartbeat</strong><small>Reports bounded counters to the connected local node.</small></span>
            <input type="checkbox" checked={draft.minerHeartbeat} onChange={(event) => update('minerHeartbeat', event.target.checked)} disabled={minerStatus.running} />
          </label>

          <div className="button-row mining-actions">
            <button className="secondary-button" type="submit" disabled={busy || minerStatus.running}>Save mining settings</button>
            {saved && <span className="saved-message">Saved</span>}
            <button
              className="primary-button"
              type="button"
              onClick={minerStatus.running ? onStop : onStart}
              disabled={busy || (!minerStatus.running && (!configured || !nodeReady || privateBlocked))}
            >
              {busy ? 'Working…' : minerStatus.running ? 'Stop miner' : 'Start mining'}
            </button>
          </div>
        </form>

        <aside className="mining-side-stack">
          <section className="panel">
            <div className="panel-header"><div><span className="eyebrow">Live telemetry</span><h3>Mining performance</h3></div></div>
            <div className="mining-metric-grid">
              <div><span>Hashrate</span><strong>{formatHashrate(telemetry.hashesPerSec)}</strong></div>
              <div><span>Attempts</span><strong>{telemetry.attempts.toLocaleString()}</strong></div>
              <div><span>Templates</span><strong>{telemetry.templatesReceived.toLocaleString()}</strong></div>
              <div><span>Accepted blocks</span><strong className="success-text">{telemetry.submitsAccepted.toLocaleString()}</strong></div>
              <div><span>Rejected submits</span><strong className={telemetry.submitsRejected ? 'warning-text' : ''}>{telemetry.submitsRejected.toLocaleString()}</strong></div>
              <div><span>Stale skips</span><strong>{telemetry.templatesSkippedStale.toLocaleString()}</strong></div>
            </div>
            <div className="detail-list compact-details mining-runtime-details">
              <div><span>PID</span><strong>{minerStatus.pid ?? '—'}</strong></div>
              <div><span>Uptime</span><strong>{minerStatus.uptimeSeconds != null ? `${minerStatus.uptimeSeconds}s` : '—'}</strong></div>
              <div><span>Backend</span><strong>{telemetry.backend ?? 'cpu'}</strong></div>
              <div><span>Workers</span><strong>{telemetry.workers ?? draft.minerThreads}</strong></div>
              <div><span>Last event</span><strong>{telemetry.lastEvent ?? '—'}</strong></div>
              <div><span>Last reject</span><strong>{telemetry.lastRejectCode ?? '—'}</strong></div>
              <div><span>Template height</span><strong>{telemetry.lastTemplateHeight ?? '—'}</strong></div>
              <div><span>Accepted height</span><strong>{telemetry.lastAcceptedHeight ?? '—'}</strong></div>
            </div>
          </section>

          <section className="panel miner-log-panel">
            <div className="panel-header">
              <div><span className="eyebrow">Bounded process output</span><h3>Miner log</h3></div>
              <button className="secondary-button compact-button" onClick={onClearLogs} disabled={busy || logs.length === 0}>Clear</button>
            </div>
            <div className="miner-log-list">
              {logs.length === 0 && <div className="empty-state compact-empty">No miner output yet.</div>}
              {logs.slice(-120).map((entry) => (
                <div className={`miner-log-line ${entry.stream.includes('stderr') ? 'error-line' : ''}`} key={entry.sequence}>
                  <time>{new Date(entry.timestampMs).toLocaleTimeString()}</time>
                  <span>{entry.stream}</span>
                  <code>{entry.message}</code>
                </div>
              ))}
            </div>
          </section>
        </aside>
      </section>
    </>
  )
}
