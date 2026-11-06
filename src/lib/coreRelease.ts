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

export function isFrozenCoreReleaseIdentity(identity: CoreReleaseIdentity | null | undefined): boolean {
  return identity?.freezeState === 'frozen'
    && typeof identity.sourceCommit === 'string'
    && identity.sourceCommit.length === 40
    && typeof identity.sourceTree === 'string'
    && identity.sourceTree.length === 40
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
  return matchesCoreNodeProvenance(proof, registry?.v24)
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
  return matchesCoreMinerProvenance(proof, registry?.v24)
}
