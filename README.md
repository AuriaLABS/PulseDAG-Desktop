# PulseDAG Desktop

PulseDAG Desktop is the native operator client for running and observing a local PulseDAG node. It is intentionally separate from the public read-only explorer.

## Current capabilities

- Tauri 2 desktop shell with a Rust supervision backend.
- React 19, TypeScript and Vite frontend.
- Dark and light PulseDAG themes.
- Native file and directory dialogs for selecting `pulsedagd`, persistent state and diagnostic output.
- Manual path selection and automatic discovery of `pulsedagd` beside the app, in `./bin`, through `PULSEDAGD_PATH`, or on `PATH`.
- Native binary validation for the exact file name, executable permissions, size and SHA-256 digest.
- Verification of original v2.3.0 release archives against the digest and exact source commit published by the approved PulseDAG GitHub release.
- Supervised process start, stop and restart.
- Captured stdout, stderr and desktop lifecycle messages with a bounded in-memory log buffer.
- Redacted JSON diagnostic export covering runtime state, binary evidence, loopback health and captured logs.
- Loopback-only RPC health checks against `GET /health`.
- Native read-only observability through exact v2.3.0 API routes for node status, synchronization, mempool, PoW health and recent blocks.
- Network workspace with chain identity, peer count, P2P mode, convergence gaps, readiness evidence and ledger pressure.
- Live DAG workspace with the recent frontier, selected-tip evidence and exact recent-block fields.
- Persistent non-sensitive preferences for executable path, data directory, RPC origin and configuration profile.
- CI for TypeScript, frontend production build, Rust validation and the observability route allowlist.

## Read-only observability boundary

PulseDAG Desktop reads only the following exact local routes:

- `GET /api/v1/status`
- `GET /api/v1/blocks/recent?limit=20`
- `GET /api/v1/sync/status`
- `GET /api/v1/mempool`
- `GET /api/v1/pow/health`

The native adapter:

1. Reuses the loopback-only RPC origin validation.
2. Rejects every path outside the explicit observability allowlist.
3. Disables HTTP redirects.
4. Applies connection and request timeouts.
5. Rejects responses larger than 1 MiB.
6. Requires status and recent blocks, while allowing sync, mempool and PoW panels to degrade independently.

The approved status contract exposes peer count and P2P mode, not individual peer identities or addresses. The desktop therefore does not infer a peer table. The recent-block route exposes parent count but not parent hashes, so the Live DAG page presents an exact frontier timeline instead of drawing synthetic edges.

## Release verification boundary

The release verifier accepts an original `pulsedagd-v2.3.0-<target>.tar.gz` or `.zip` asset. The Rust backend:

1. Hashes the selected archive locally with SHA-256.
2. Queries the official `AuriaLABS/PulseDAG` GitHub release for tag `v2.3.0`.
3. Requires the published release to target the approved source commit `7e43225f01ac05d15e5f1e3f1550d7850bf18cbc`.
4. Locates the exact release asset by file name.
5. Compares the local archive digest with GitHub's published asset digest.

This proves that the selected archive matches the approved release asset. It does not prove the origin of an extracted executable obtained separately. Keep the archive and extract the node directly from the verified copy.

## Node launch boundary

The desktop backend accepts the `dev`, `local` and `private` PulseDAG configuration profiles. Before launching the node it:

1. Validates the selected `pulsedagd` file.
2. Requires a persistent data directory.
3. Removes all inherited `PULSEDAG_*` variables.
4. Restores the non-PulseDAG parent environment.
5. Supplies an explicit profile, RPC bind, RocksDB path and P2P identity path.
6. Forces `PULSEDAG_ADMIN_ENABLED=false`.
7. Restricts RPC to `localhost`, `127.0.0.1` or `::1` over HTTP.

## Diagnostic export boundary

The diagnostic bundle is written only to a user-selected `.json` file. Before writing, the backend replaces:

- the selected executable path;
- the persistent data directory;
- the current home or user-profile directory;
- matching path fragments captured in stdout, stderr or desktop lifecycle logs.

The bundle contains no wallet, signing, mining or operator-token fields. Users should still review the JSON before sharing it because application logs can contain arbitrary text emitted by the node or operating system.

## Security boundary

- Administrative, wallet, mining and transaction-submission capabilities are not exposed.
- Credentials and operator tokens are not accepted by the frontend or stored in local preferences.
- RPC URLs containing credentials, query strings, fragments or non-loopback hosts are rejected.
- The frontend receives explicit status objects and invokes narrowly scoped Tauri commands.
- Graceful stop uses `SIGTERM` on Unix, followed by a forced stop after five seconds when required.
- Launch-on-startup remains disabled until crash recovery and ownership policies are defined.

## Development

Prerequisites:

- Node.js 22 or newer
- Rust stable
- Tauri 2 platform dependencies

Install dependencies and start the native application:

```bash
npm install
npm run tauri dev
```

Run frontend checks:

```bash
npm run typecheck
npm run build
```

Run the Rust check and observability allowlist test:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml observability_paths_are_exactly_allowlisted
```

## Next milestone

1. Add bounded block detail and transaction drill-down views using approved GET routes.
2. Bind extracted binaries to verified archive provenance without storing privileged secrets.
3. Add diagnostic schema tests and user-selectable log windows.
4. Add Windows and Linux packaging workflows.
