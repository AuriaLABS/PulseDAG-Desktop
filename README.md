# PulseDAG Desktop

PulseDAG Desktop is the native operator client for running and observing a local PulseDAG node. It is intentionally separate from the public read-only explorer.

## Current milestone

This initial scaffold provides:

- Tauri 2 desktop shell with a small Rust command bridge.
- React 19, TypeScript and Vite frontend.
- Dark and light PulseDAG visual themes.
- Navigation for overview, node, network, live DAG, logs and settings.
- Local non-sensitive node preferences.
- First-run readiness and security-boundary views.
- CI for TypeScript, frontend production build and Rust validation.

Node process supervision, live RPC validation and log streaming are represented in the interface but remain disabled until they are implemented in Rust.

## Security boundary

PulseDAG Desktop is designed so that sensitive operator capabilities remain in the native backend:

- Do not expose administrative, wallet or mining RPC to a browser.
- Do not place credentials or operator tokens in frontend environment variables or local storage.
- Frontend code should receive only explicit status objects and invoke narrowly scoped Tauri commands.
- Wallet, signing, transaction submission and mining controls are outside this milestone.

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

## Planned next milestone

1. Native file picker and validation for the `pulsedagd` executable.
2. Safe process start, stop and restart commands.
3. Loopback-only RPC health checks against the approved PulseDAG API contract.
4. Structured local log streaming and diagnostic export.
5. Adapt selected read-only components from PulseDAG Explorer for local node observability.
