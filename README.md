# PulseDAG Desktop

PulseDAG Desktop is the native operator client for running, observing and locally mining against a PulseDAG node. It is intentionally separate from the public read-only explorer.

## Current capabilities

- Tauri 2 desktop shell with Rust supervision backends for `pulsedagd` and the external `pulsedag-miner`.
- React 19, TypeScript and Vite frontend.
- Dark and light PulseDAG themes.
- Native file and directory dialogs for selecting the node, miner, persistent state and diagnostic output.
- Manual path selection and automatic discovery of `pulsedagd` and `pulsedag-miner` beside the app, in `./bin`, through their dedicated environment variables, or on `PATH`.
- Native binary validation for exact file names, executable permissions, size and SHA-256 digest.
- Verification of original v2.3.0 node release archives against the digest and exact source commit published by the approved PulseDAG GitHub release.
- Byte-for-byte linkage between the selected node executable and `pulsedagd` stored inside the approved release archive.
- Supervised node start, stop and restart, with approved provenance required for the private profile.
- Supervised standalone CPU mining for development and local profiles through a fixed official CLI surface.
- Miner telemetry for hashrate, attempts, templates, accepted blocks, rejected submissions, stale work and recent process output.
- Captured stdout, stderr and desktop lifecycle messages with bounded, separate node and miner log buffers.
- Redacted JSON diagnostic export covering node runtime state, binary evidence, release provenance, loopback health and captured node logs.
- Loopback-only RPC health checks against `GET /health`.
- Native read-only observability through exact v2.3.0 API routes for node status, synchronization, mempool, PoW health and recent blocks.
- Network workspace with chain identity, peer count, P2P mode, convergence gaps, readiness evidence and ledger pressure.
- Live DAG workspace with the recent frontier, selected-tip evidence, real block relations, confirmed block transactions and bounded transaction lookup.
- Persistent non-secret preferences for node and miner paths, public reward address, CPU limits, data directory, RPC origin and configuration profile.
- CI for TypeScript, frontend production build, Rust validation, provenance guards, miner command guards and read-only route guards.

## Read-only observability boundary

PulseDAG Desktop continuously reads only the following exact local routes:

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

The approved status contract exposes peer count and P2P mode, not individual peer identities or addresses. The desktop therefore does not infer a peer table. The recent-block route exposes parent count but not parent hashes, so the frontier itself remains an exact timeline rather than a synthetic graph.

## Bounded entity drill-down

Block and transaction inspection uses only identifiers returned by the node and accepts exactly 64 hexadecimal characters. The backend constructs only these routes:

- `GET /api/v1/blocks/<64-hex-hash>/overview`
- `GET /api/v1/blocks/<64-hex-hash>/transactions?limit=100&offset=0`
- `GET /api/v1/txs/<64-hex-txid>/lookup`

The desktop does not accept free-form paths, query strings or arbitrary entity identifiers. Block transactions are capped at the first 100 records and the interface reports when the node says more records exist. Parent, child, block and transaction navigation remains inside the same guarded commands.

## Standalone mining boundary

Mining remains external to `pulsedagd`. Desktop starts the official `pulsedag-miner` process and supplies only a closed, typed argument set:

- loopback node origin;
- public reward address;
- canonical `cpu` backend;
- bounded CPU thread count;
- bounded attempts per template;
- loop sleep and refresh-before-expiry thresholds;
- optional worker ID;
- heartbeat enabled or disabled.

The backend accepts only executables named `pulsedag-miner` or `pulsedag-miner.exe`, verifies executable permissions and SHA-256 before launch, removes inherited `PULSEDAG_*` variables, requires the local node RPC to be reachable and never accepts free-form command arguments.

Development and local profiles can mine in this milestone. Private-profile mining is deliberately blocked until the selected miner executable can be linked byte-for-byte to an approved `pulsedag-miner-v2.3.0-<target>` release archive. The CPU backend is the consensus reference. GPU mining is not enabled by Desktop.

Only a public reward address is stored in local preferences and passed to the miner. Desktop does not request, store or transmit a seed phrase, private key, wallet password or signing material. The standalone miner requests templates and submits solved blocks itself; Desktop does not construct blocks or submit mining RPC payloads.

## Release verification boundary

The node release verifier accepts an original `pulsedagd-v2.3.0-<target>.tar.gz` or `.zip` asset. The Rust backend:

1. Hashes the selected archive locally with SHA-256.
2. Queries the official `AuriaLABS/PulseDAG` GitHub release for tag `v2.3.0`.
3. Requires the published release to target the approved source commit `7e43225f01ac05d15e5f1e3f1550d7850bf18cbc`.
4. Locates the exact release asset by file name.
5. Compares the local archive digest with GitHub's published asset digest.

## Node binary provenance boundary

After the node archive is approved, the desktop can link the selected executable to the archive without extracting or running any file. The native backend:

1. Opens the already-approved archive as ZIP or TAR.GZ.
2. Requires the archive target to match the current operating system and architecture.
3. Accepts only the exact release root containing `pulsedagd` or `pulsedagd.exe`, `README.md` and `INSTALL_BINARIES_V2_3_0.md`.
4. Rejects absolute paths, `.` or `..` components, links, unsupported entry types, duplicate files, unexpected files and oversized entries.
5. Streams the embedded binary through SHA-256 with a 256 MiB limit.
6. Re-hashes the selected executable and requires identical digest and size.
7. Stores only the resulting hashes, release identity and canonical executable path in native memory for the current desktop session.

The node executable is hashed again before launch. Replacing or modifying it invalidates the proof. The `private` node profile refuses to start without a current approved proof. The `dev` and `local` profiles remain available for locally built binaries and record `unverified-development` in the desktop lifecycle log.

## Node launch boundary

The desktop backend accepts the `dev`, `local` and `private` PulseDAG configuration profiles. Before launching the node it:

1. Validates the selected `pulsedagd` file and rechecks any active provenance proof.
2. Requires approved release provenance for the private profile.
3. Requires a persistent data directory.
4. Removes all inherited `PULSEDAG_*` variables.
5. Restores the non-PulseDAG parent environment.
6. Supplies an explicit profile, RPC bind, RocksDB path and P2P identity path.
7. Forces `PULSEDAG_ADMIN_ENABLED=false`.
8. Restricts RPC to `localhost`, `127.0.0.1` or `::1` over HTTP.

## Diagnostic export boundary

The diagnostic bundle is written only to a user-selected `.json` file. Before writing, the backend replaces:

- the selected node executable path;
- the persistent data directory;
- the current home or user-profile directory;
- matching path fragments captured in node stdout, stderr or lifecycle logs.

Schema version 2 includes node release tag, source commit, target, archive digest and embedded binary digest when an approved proof exists. It does not include the local archive path, node executable path, miner configuration, reward address or miner logs. The bundle contains no seed, private key, signing material, wallet password or operator token. Users should still review the JSON before sharing it because application logs can contain arbitrary text emitted by the node or operating system.

## Security boundary

- Administrative RPC, wallet management and general transaction-signing capabilities are not exposed.
- Mining is limited to supervision of the official external miner through fixed local arguments; Desktop does not expose a generic command runner.
- Credentials, seeds, private keys, wallet passwords and operator tokens are not accepted by the frontend or stored in local preferences.
- RPC URLs containing credentials, query strings, fragments or non-loopback hosts are rejected.
- Entity identifiers are fixed-length hexadecimal values and cannot inject paths, queries or fragments.
- Node release archives are inspected in memory and are never automatically extracted or executed.
- The frontend receives explicit status objects and invokes narrowly scoped Tauri commands.
- Graceful stop uses `SIGTERM` on Unix, followed by a forced stop after five seconds when required.
- Launch-on-startup remains disabled until crash recovery and ownership policies are defined.
- Pool coordination, shares, payouts, accounting and remote mining endpoints remain outside scope.

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

Run the Rust checks and security tests:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml observability_paths_are_exactly_allowlisted
cargo test --manifest-path src-tauri/Cargo.toml entity_
cargo test --manifest-path src-tauri/Cargo.toml runtime_
cargo test --manifest-path src-tauri/Cargo.toml miner_
cargo test --manifest-path src-tauri/Cargo.toml provenance_
cargo test --manifest-path src-tauri/Cargo.toml diagnostic_
```

## Next milestone

1. Link `pulsedag-miner` byte-for-byte to its approved release archive and enable private-profile mining.
2. Exercise a real Windows node-plus-miner flow and record accepted/rejected submission evidence.
3. Define crash recovery and process ownership before enabling launch on startup.
