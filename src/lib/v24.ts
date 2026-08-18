import { invoke } from '@tauri-apps/api/core'

export type V24CandidateBinaryKind = 'node' | 'miner'

export type V24CandidateArchiveInspection = {
  archiveName: string
  archiveSha256: string
  archiveSizeBytes: number
  binaryKind: V24CandidateBinaryKind
  releaseTag: 'v2.4.0'
  sourceCommit: string
  target: string
  embeddedPath: string
  embeddedBinarySha256: string
  embeddedBinarySizeBytes: number
  structurallyValid: boolean
  approved: false
  message: string
}

export async function inspectV24CandidateArchive(
  path: string,
  binaryKind: V24CandidateBinaryKind,
): Promise<V24CandidateArchiveInspection> {
  return invoke<V24CandidateArchiveInspection>('inspect_v2_4_candidate_archive', {
    path,
    binaryKind,
  })
}
