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
- Persistent non-sensitive preferences for executable path, data directory, RPC origin and configuration profile.
- Overview, node, network, Live DAG, logs and settings workspaces.
- CI for TypeScript, frontend production build and Rust validation.

Network and Live DAG pages remain placeholders until the approved read-only node adapters are integrated.

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

Run the Rust check:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Next milestone

1. Read-only node status, synchronization and peer adapters.
2. Adapt selected Live DAG and entity views from PulseDAG Explorer.
3. Bind extracted binaries to verified archive provenance without storing privileged secrets.
4. Add diagnostic schema tests and user-selectable log windows.
5. Windows and Linux packaging workflows.
