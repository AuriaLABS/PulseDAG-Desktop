# Native packaging and final v2.4 release rehearsal

PulseDAG Desktop builds native, unsigned CI artifacts for Windows x86_64 and Linux x86_64.

## Produced bundles

- Linux: Debian package (`.deb`) and AppImage.
- Windows: NSIS setup executable and MSI package.
- Every artifact set includes `SHA256SUMS.txt`.
- Every artifact set includes `OFFICIAL_NODE_RELEASE_EVIDENCE.json` and `OFFICIAL_MINER_RELEASE_EVIDENCE.json` for the exact final PulseDAG v2.4.0 release assets verified before packaging.

The workflow does not publish a GitHub release and does not sign any installer. Artifact names explicitly include `unsigned`, and CI retains them for 14 days.

## Final v2.4.0 release boundary

Native packaging is pinned to the Task31 release decision:

- tag: `v2.4.0`;
- source SHA: `876b48826a3875b729888edb88e2b0eea15bb717`;
- source tree: `f41f65bc5c5da3a44903b84f0e0f7186df2b64a8`;
- private network profile: `private-testnet-v2.4.0`;
- chain ID: `pulsedag-private-v2.4.0`;
- protocol consensus mode: `ghostdag_v1`;
- public-testnet readiness: false;
- 30-day public-testnet clock: not started;
- contracts: disabled.

The final-release verifier requires the exact published 21-asset allowlist, the frozen archive SHA-256 values, GitHub asset digests, per-archive `.sha256` and JSON manifests, `SHA256SUMS.txt`, and `release-provenance.json` with native smoke verification. A source, target, layout, digest, manifest, or provenance mismatch fails closed.

## Native package rehearsal

Before building the Desktop packages, each native runner:

1. Verifies and downloads the final v2.4.0 `pulsedagd` archive for its target.
2. Verifies and downloads the final v2.4.0 `pulsedag-miner` archive for its target.
3. Extracts both archives using native runner tooling.
4. Runs `v2_4_final_node_release_rehearsal` and confirms the extracted node is byte-for-byte identical to the verified archive member.
5. Runs `v2_4_final_miner_release_rehearsal` and confirms the extracted miner is byte-for-byte identical to the verified archive member.
6. Requires both proofs to satisfy the frozen Task31 v2.4.0 provenance gate.
7. Runs bounded `pulsedagd --version` and `pulsedag-miner --help` smoke checks without starting a network.
8. Typechecks the frontend and builds the unsigned native Desktop bundles.
9. Stages the Desktop installers together with final node/miner release evidence and checksums.

The separate `v2.4 Private Mining Smoke` workflow then validates real binary interoperability on Linux and Windows. It starts the final node in isolated single-node private mode with P2P disabled and loopback-only RPC, starts the final miner with one CPU worker and bounded work, and requires node health plus miner `template_received` and `mining_result` telemetry. This smoke does not authorize or start public testnet.

## Legacy v2.3 compatibility

Development/local compatibility paths may still inspect and bind the previously approved v2.3.0 binaries. Those proofs are legacy compatibility evidence only. They do not authorize the v2.4.0 `private` node or miner launch paths.

## Manual builds

Linux:

```bash
npm install
npm run tauri icon src-tauri/icons/icon.png
npm run tauri build -- --bundles deb,appimage
```

Windows:

```powershell
npm install
npm run tauri icon src-tauri/icons/icon.png
npm run tauri build -- --bundles nsis,msi
```

Tauri platform dependencies are required. Generated bundles are under `src-tauri/target/release/bundle/`.

## Promotion boundary

CI artifacts are engineering evidence, not public Desktop releases. Promotion still requires:

- native installation and UI smoke tests;
- real start, health, observability, mining, stop, and restart checks on the target operator machine using final v2.4.0 node/miner proofs;
- code-signing ownership and key-management decisions;
- a reviewed Desktop release version and changelog;
- explicit approval to publish.

None of these packaging or private-mining checks starts the PulseDAG public-testnet clock, enables contracts, or adds wallet custody/private-key handling to Desktop.
