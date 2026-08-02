# PulseDAG Desktop

PulseDAG Desktop is the native operator client for running and observing a local PulseDAG node. It is intentionally separate from the public read-only explorer.

## Current capabilities

- Tauri 2 desktop shell with a Rust supervision backend.
- React 19, TypeScript and Vite frontend.
- Dark and light PulseDAG themes.
- Manual path selection and automatic discovery of `pulsedagd` beside the app, in `./bin`, through `PULSEDAGD_PATH`, or on `PATH`.
- Native binary validation for the exact file name, executable permissions, size and SHA-256 digest.
- Supervised process start, stop and restart.
- Captured stdout, stderr and desktop lifecycle messages with a bounded in-memory log buffer.
- Loopback-only RPC health checks against `GET /health`.
- Persistent non-sensitive preferences for executable path, data directory, RPC origin and configuration profile.
- Overview, node, network, Live DAG, logs and settings workspaces.
- CI for TypeScript, frontend production build and Rust validation.

Network and Live DAG pages remain placeholders until the approved read-only node adapters are integrated.

## Node launch boundary

The desktop backend accepts the `dev`, `local` and `private` PulseDAG configuration profiles. Before launching the node it:

1. Validates the selected `pulsedagd` file.
2. Requires a persistent data directory.
3. Removes all inherited `PULSEDAG_*` variables.
4. Restores the non-PulseDAG parent environment.
5. Supplies an explicit profile, RPC bind, RocksDB path and P2P identity path.
6. Forces `PULSEDAG_ADMIN_ENABLED=false`.
7. Restricts RPC to `localhost`, `127.0.0.1` or `::1` over HTTP.

The SHA-256 shown in the application is local evidence, not release-signature verification. Approved release checksums and signatures must still be verified through the PulseDAG release process.

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

1. Native file and directory picker integration.
2. Version and approved-release digest verification for `pulsedagd`.
3. Read-only node status, synchronization and peer adapters.
4. Adapt selected Live DAG and entity views from PulseDAG Explorer.
5. Structured diagnostic export with explicit redaction rules.
6. Windows and Linux packaging workflows.
