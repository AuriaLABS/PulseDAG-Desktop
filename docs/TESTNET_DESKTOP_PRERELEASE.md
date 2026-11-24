# PulseDAG Desktop testnet prerelease channel

Status: **PREVIEW ONLY — NOT PUBLIC TESTNET READY** (Core #781/#1418 NO-GO).

## Version separation

| Channel | Version | Tauri identifier | Use |
| --- | --- | --- | --- |
| Final/mainnet Desktop | `0.1.0` | `io.aurialabs.pulsedag.desktop` | Reserved for final independently approved Core v3.0.0 mainnet |
| Testnet Desktop | `0.1.0-testnet.1` | `io.aurialabs.pulsedag.desktop.testnet` | Separate, clearly labelled prerelease build for later public testnet validation |

This version does **not** imply that a valid Core v3 node, miner, wallet,
testnet genesis or network launch exists. It does **not** publish a GitHub
Release. Until the exact Core candidate, network identity and
`GO_PUBLIC_TESTNET` are verified under Core #781, there is no public testnet
Desktop release. The prerelease version can increment to `testnet.2`,
`testnet.3`, etc., when required by actual build and protocol changes.

Test coins will have **no monetary value**, and testnet may reset under its
published reset policy. Mainnet requires its own distinct `GO_V3_MAINNET`
after meaningful public-testnet evidence, not just a successful testnet build.

## What is separated already

- OS bundle identifier, product name, window title and installer version through
  `src-tauri/tauri.testnet.conf.json`.
- Local node preferences and theme storage keys when
  `VITE_PULSEDAG_DESKTOP_CHANNEL=testnet`.
- Prominent persistent **INTERNAL PREVIEW ONLY / NO-GO** banner.
- The ordinary `package.json`, Cargo and Tauri config on `main` retain
  `0.1.0`; the testnet build rewrites only an ephemeral working copy of
  npm/Cargo versions, applying a Tauri config override.
- Manual CI preview builds run without artifact upload, release publication
  or monetary/contract network activation.

The existing v2.4 local/private development path is not a real v3 public
testnet, and passing its compatibility checks cannot satisfy the v3 GO gate.

## Internal build verification (not for distribution)

In a disposable Git checkout:

```sh
npm install
node .github/scripts/check-desktop-version.mjs
node .github/scripts/prepare-testnet-version.mjs check
node .github/scripts/prepare-testnet-version.mjs prepare
node .github/scripts/prepare-testnet-version.mjs verify-prepared
VITE_PULSEDAG_DESKTOP_CHANNEL=testnet npm run tauri build -- --config src-tauri/tauri.testnet.conf.json
```

This mutates the disposable checkout's `package.json` and
`src-tauri/Cargo.toml`; **never commit those changes onto `main`**. The
Tauri override supplies the prerelease version and separate product identity.
Builds may still fail pending native toolchain and installer requirements.

The `Desktop testnet preview metadata (NO-GO)` workflow checks stable and
testnet metadata on PRs. Its manual dispatch requires
`INTERNAL_PREVIEW_ONLY`; any resulting unsigned bundles stay inside the
ephemeral CI runner and **are not uploaded or released**.

## Public prerelease gate — still pending

Only proceed after all of these are evidenced on the **exact** Core testnet
candidate:

1. Core #781 records `GO_PUBLIC_TESTNET`, with independent genesis, chain ID,
   signing domain, source/tree, signed/hashed node-miner-wallet artifacts,
   bounded RPC/P2P profile and operator/incident/STOP plan (Core #1049/#1418).
2. Desktop consumes the approved testnet protocol/identity instead of v2.4,
   keeps mainnet launch fail-closed and proves that testnet identity cannot be
   replayed or relabelled as mainnet.
3. Windows/Linux clean installation, side-by-side stable install, isolated
   persistence, native provenance, rollback and testnet-only smoke tests pass.
4. Public testnet messaging, no-value test coins, reset policy and contract
   **INACTIVE** state are visible; no custody or GPU production claim.
5. Release owner separately approves the exact prerelease package and its
   distribution evidence. Testnet GO is **not** mainnet GO.

Never automatically promote `0.1.0-testnet.1` to final `0.1.0`.
