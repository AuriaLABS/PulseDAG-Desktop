# Native packaging and official-release rehearsal

PulseDAG Desktop builds native, unsigned CI artifacts for Windows x86_64 and Linux x86_64.

## Produced bundles

- Linux: Debian package (`.deb`) and AppImage.
- Windows: NSIS setup executable and MSI package.
- Every artifact set includes `SHA256SUMS.txt`.
- Every artifact set includes `OFFICIAL_NODE_RELEASE_EVIDENCE.json`, which records the pinned PulseDAG node release identity used by the native rehearsal.

The workflow does not publish a GitHub release and does not sign any installer. The artifact names explicitly include `unsigned`, and CI retains them for 14 days.

## Official node release rehearsal

Before building the desktop packages, each native runner:

1. Queries the official `AuriaLABS/PulseDAG` release tagged `v2.3.0`.
2. Requires the release to point to source commit `7e43225f01ac05d15e5f1e3f1550d7850bf18cbc`.
3. Selects only the exact asset for the runner target.
4. Validates the GitHub asset size and SHA-256 digest, with the published `.sha256` asset as a fallback.
5. Downloads with a 512 MiB bound and rejects unexpected release URLs or redirect hosts.
6. Extracts the archive using the native runner tooling.
7. Runs the ignored Rust test `provenance_official_release_rehearsal`.
8. Confirms that the extracted executable is byte-for-byte identical to the archive member.
9. Confirms that the private profile rejects the executable before provenance is installed and accepts it after the current proof is installed.

The rehearsal deliberately stops at launch admission. It does not start a P2P node or contact a testnet from CI.

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

CI artifacts are engineering evidence, not public releases. Promotion requires:

- native installation and UI smoke tests;
- real start, health, observability, stop and restart checks with the approved node binary;
- code-signing ownership and key-management decisions;
- a reviewed release version and changelog;
- explicit approval to publish.
