# PulseDAG Desktop

PulseDAG Desktop is the native operator client for running, observing and locally mining against a PulseDAG node. It remains separate from the public read-only explorer.

## Current capabilities

- Tauri 2 desktop shell with independent Rust supervisors for `pulsedagd` and the official external `pulsedag-miner`.
- React 19, TypeScript and Vite frontend with dark and light themes.
- Native selection, discovery and SHA-256 validation of the node and miner executables.
- Persistent node state with a loopback-only RPC boundary.
- Approved v2.3.0 release verification for the node and miner archives.
- Byte-for-byte linkage between each selected executable and the matching member of its approved archive.
- Supervised node start, stop and restart.
- Supervised standalone CPU mining with bounded configuration and typed telemetry.
- Private-profile node and miner launch only when their separate provenance proofs are current.
- Development and local profiles for locally built, explicitly unverified binaries.
- Read-only network, synchronization, mempool, PoW and DAG observability.
- Bounded node and miner logs, plus redacted node diagnostic export.
- Unsigned Windows MSI/NSIS and Linux Debian/AppImage packaging workflows.

## Read-only observability boundary

PulseDAG Desktop continuously reads only these exact local routes:

- `GET /api/v1/status`
- `GET /api/v1/blocks/recent?limit=20`
- `GET /api/v1/sync/status`
- `GET /api/v1/mempool`
- `GET /api/v1/pow/health`

Block and transaction inspection constructs only these bounded routes from fixed-length hexadecimal identifiers:

- `GET /api/v1/blocks/<64-hex-hash>/overview`
- `GET /api/v1/blocks/<64-hex-hash>/transactions?limit=100&offset=0`
- `GET /api/v1/txs/<64-hex-txid>/lookup`

The native adapter rejects non-loopback origins, credentials, query injection, fragments, redirects, oversized responses and routes outside the allowlist.

## Standalone mining boundary

Mining remains external to `pulsedagd`. Desktop starts the official `pulsedag-miner` process and supplies only a closed, typed argument set:

- loopback node origin;
- public reward address;
- canonical `cpu` backend;
- bounded CPU thread count and attempts per template;
- bounded loop sleep and refresh-before-expiry values;
- optional worker ID;
- heartbeat enabled or disabled.

The backend accepts only executables named `pulsedag-miner` or `pulsedag-miner.exe`, verifies executable permissions and SHA-256 before launch, removes inherited `PULSEDAG_*` variables and requires the local node RPC to be reachable. Desktop never accepts free-form command arguments.

Development and local profiles may use locally built miner binaries and record an unverified-development boundary. Private-profile mining uses a separate native command that refuses to start unless the selected miner is linked to an approved v2.3.0 miner archive in the current desktop session.

Only a public reward address is stored in local preferences. Desktop does not request, store or transmit a seed phrase, private key, wallet password or signing material. The external miner requests templates and submits solved blocks itself; Desktop does not construct blocks or submit mining payloads.

GPU mining, pool coordination, shares, payouts, accounting and remote mining endpoints remain outside the current scope.

## Approved release verification

Node and miner verification are independent. The verifier accepts only an original asset whose name matches one of these patterns for the current native target:

- `pulsedagd-v2.3.0-<target>.tar.gz` or `.zip`;
- `pulsedag-miner-v2.3.0-<target>.tar.gz` or `.zip`.

For each selected archive, the Rust backend:

1. Resolves a regular local file within the 512 MiB limit.
2. Computes its SHA-256 digest.
3. Queries the official `AuriaLABS/PulseDAG` GitHub release tagged `v2.3.0`.
4. Requires the release to identify approved source commit `7e43225f01ac05d15e5f1e3f1550d7850bf18cbc`.
5. Locates the exact asset and compares the local digest with the published digest or checksum asset.

A node archive cannot authorize a miner and a miner archive cannot authorize a node. Their native trust registries, commands and CI evidence are separate.

## Binary provenance boundary

After an archive is approved, Desktop reads it without extracting or executing anything. Node and miner inspection apply the same strict structure:

1. The archive target must match the current OS and architecture.
2. Windows uses ZIP; Linux and macOS use TAR.GZ.
3. The exact versioned root may contain only the matching executable, `README.md` and `INSTALL_BINARIES_V2_3_0.md`.
4. Absolute paths, `.` or `..`, links, unsupported entry types, duplicate entries and unexpected files are rejected.
5. The embedded executable is streamed through SHA-256 with a 256 MiB limit.
6. The selected executable is re-hashed and must match the embedded digest and size exactly.
7. Only the resulting hashes, release identity and canonical executable path are retained in native memory for the current session.

Both executables are hashed again before private launch. Replacing or modifying either file invalidates only its corresponding proof. Restarting Desktop requires the archive-to-executable linkage to be established again.

## Node launch boundary

Before launching `pulsedagd`, Desktop:

1. validates the executable and rechecks active provenance;
2. requires node provenance for the `private` profile;
3. creates and normalizes the persistent data directory;
4. removes inherited `PULSEDAG_*` variables;
5. supplies the selected profile, loopback RPC bind, RocksDB path and P2P identity path;
6. forces administrative RPC off and clears the CORS allowlist.

Windows verbatim data paths are converted to normal drive or UNC paths before being passed to RocksDB.

## Logs and diagnostics

Node and miner processes have separate bounded in-memory log buffers. Miner output is parsed only when it uses the official `miner_telemetry` format, producing typed counters for hashrate, attempts, templates, accepted and rejected submissions, stale work and recent heights.

The JSON diagnostic export covers the node runtime, binary evidence, node release provenance, loopback health and redacted node logs. It excludes the reward address, miner configuration, miner logs, local archive paths, wallet material and operator tokens.

## Security boundary

- Administrative RPC, wallet management and transaction signing are not exposed.
- Mining is limited to supervision of the official external miner through fixed local arguments.
- Credentials, seeds, private keys, wallet passwords and operator tokens are not accepted.
- RPC must use HTTP on `localhost`, `127.0.0.1` or `::1` and contain no credentials, query or fragment.
- Release archives are inspected in memory and are never automatically extracted or executed.
- The frontend receives typed results and invokes narrowly scoped Tauri commands.
- Graceful stop uses `SIGTERM` on Unix and a bounded forced-stop fallback.
- Launch-on-startup remains disabled until crash recovery and process ownership policies are defined.

## Development

Prerequisites:

- Node.js 22 or newer;
- Rust stable;
- Tauri 2 platform dependencies.

```bash
npm install
npm run tauri dev
npm run typecheck
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml observability_paths_are_exactly_allowlisted
cargo test --manifest-path src-tauri/Cargo.toml entity_
cargo test --manifest-path src-tauri/Cargo.toml runtime_
cargo test --manifest-path src-tauri/Cargo.toml miner_
cargo test --manifest-path src-tauri/Cargo.toml provenance_
cargo test --manifest-path src-tauri/Cargo.toml diagnostic_
```

## Remaining promotion checks

1. Install the Windows package on a native desktop.
2. Verify and link both official v2.3.0 archives and executables.
3. Start the node and miner through the real UI.
4. Observe template, hashrate and accepted or explicitly rejected submission evidence.
5. Define crash recovery, process ownership, signing and public-release promotion.
