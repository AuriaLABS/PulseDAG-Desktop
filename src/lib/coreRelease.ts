import type { BinaryProvenance } from '../types'

export type CoreReleaseFreezeState = 'frozen' | 'pending'

export type CoreReleaseIdentity = {
  releaseTag: string
  sourceCommit: string | null
  sourceTree: string | null
  freezeState: CoreReleaseFreezeState
}

export const V24_CORE_RELEASE_IDENTITY: CoreReleaseIdentity = {
  releaseTag: 'v2.4.0',
  sourceCommit: '876b48826a3875b729888edb88e2b0eea15bb717',
  sourceTree: 'f41f65bc5c5da3a44903b84f0e0f7186df2b64a8',
  freezeState: 'frozen',
}

export const V30_CORE_RELEASE_IDENTITY: CoreReleaseIdentity = {
  releaseTag: 'v3.0.0',
  sourceCommit: null,
  sourceTree: null,
  freezeState: 'pending',
}

export function isFrozenCoreReleaseIdentity(identity: CoreReleaseIdentity): boolean {
  return identity.freezeState === 'frozen'
    && identity.sourceCommit !== null
    && identity.sourceTree !== null
}

export function matchesCoreNodeProvenance(
  proof: BinaryProvenance | null,
  identity: CoreReleaseIdentity,
): boolean {
  if (!proof?.approved || !isFrozenCoreReleaseIdentity(identity)) return false
  return proof.releaseTag === identity.releaseTag
    && proof.sourceCommit === identity.sourceCommit
    && proof.archiveName.startsWith(`pulsedagd-${identity.releaseTag}-`)
}

export function isFinalV24NodeProvenance(proof: BinaryProvenance | null): boolean {
  return matchesCoreNodeProvenance(proof, V24_CORE_RELEASE_IDENTITY)
}
