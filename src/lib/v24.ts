import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { BinaryProvenance, ReleaseVerification } from '../types'

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

function selectedPath(value: string | string[] | null): string | null {
  return typeof value === 'string' ? value : null
}

export async function selectV24ReleaseArchive(
  binaryKind: V24CandidateBinaryKind,
): Promise<string | null> {
  return selectedPath(await open({
    title: `Select the final PulseDAG v2.4.0 ${binaryKind} release archive`,
    directory: false,
    multiple: false,
    filters: [{ name: 'PulseDAG v2.4.0 release archives', extensions: ['zip', 'gz'] }],
  }))
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

export async function verifyV24ReleaseArchive(
  path: string,
  binaryKind: V24CandidateBinaryKind,
): Promise<ReleaseVerification> {
  return invoke<ReleaseVerification>('verify_v2_4_release_archive', {
    path,
    binaryKind,
  })
}

export async function bindV24NodeBinaryToVerifiedArchive(
  archivePath: string,
  executablePath: string,
): Promise<BinaryProvenance> {
  return invoke<BinaryProvenance>('bind_v2_4_node_binary_to_verified_archive', {
    archivePath,
    executablePath,
  })
}

export async function bindV24MinerBinaryToVerifiedArchive(
  archivePath: string,
  executablePath: string,
): Promise<BinaryProvenance> {
  return invoke<BinaryProvenance>('bind_v2_4_miner_binary_to_verified_archive', {
    archivePath,
    executablePath,
  })
}
