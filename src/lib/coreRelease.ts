import type { BinaryProvenance, ReleaseVerification } from '../types'

export type CoreReleaseFreezeState = 'frozen' | 'pending'

export type CoreReleaseIdentity = {
  releaseTag: string
  sourceCommit: string | null
  sourceTree: string | null
  freezeState: CoreReleaseFreezeState
}

export type CoreReleaseIdentityRegistry = {
  v24: CoreReleaseIdentity
  v30: CoreReleaseIdentity
}

function isCanonicalSha(value: string | null | undefined): boolean {
  return typeof value === 'string' && /^[0-9a-f]{40}$/.test(value)
}

export function isFrozenCoreReleaseIdentity(identity: CoreReleaseIdentity | null | undefined): boolean {
  return identity?.freezeState === 'frozen'
    && isCanonicalSha(identity.sourceCommit)
    && isCanonicalSha(identity.sourceTree)
}

export function matchesCoreReleaseArchive(
  verification: ReleaseVerification | null,
  identity: CoreReleaseIdentity | null | undefined,
  binary: 'pulsedagd' | 'pulsedag-miner',
): boolean {
  if (!verification?.approved || !identity || !isFrozenCoreReleaseIdentity(identity)) return false
  return verification.releaseTag === identity.releaseTag
    && verification.sourceCommit === identity.sourceCommit
    && verification.archiveName.startsWith(`${binary}-${identity.releaseTag}-`)
}

export function matchesCoreNodeProvenance(
  proof: BinaryProvenance | null,
  identity: CoreReleaseIdentity | null | undefined,
): boolean {
  if (!proof?.approved || !identity || !isFrozenCoreReleaseIdentity(identity)) return false
  return proof.releaseTag === identity.releaseTag
    && proof.sourceCommit === identity.sourceCommit
    && proof.archiveName.startsWith(`pulsedagd-${identity.releaseTag}-`)
}

export function isFinalV24NodeProvenance(
  proof: BinaryProvenance | null,
  registry: CoreReleaseIdentityRegistry | null,
): boolean {
  return registry?.v24.releaseTag === 'v2.4.0'
    && matchesCoreNodeProvenance(proof, registry.v24)
}

export function matchesCoreMinerProvenance(
  proof: BinaryProvenance | null,
  identity: CoreReleaseIdentity | null | undefined,
): boolean {
  if (!proof?.approved || !identity || !isFrozenCoreReleaseIdentity(identity)) return false
  return proof.releaseTag === identity.releaseTag
    && proof.sourceCommit === identity.sourceCommit
    && proof.archiveName.startsWith(`pulsedag-miner-${identity.releaseTag}-`)
}

export function isFinalV24MinerProvenance(
  proof: BinaryProvenance | null,
  registry: CoreReleaseIdentityRegistry | null,
): boolean {
  return registry?.v24.releaseTag === 'v2.4.0'
    && matchesCoreMinerProvenance(proof, registry.v24)
}
