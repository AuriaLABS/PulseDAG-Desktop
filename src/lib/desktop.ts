import { invoke } from '@tauri-apps/api/core'
import type { DesktopBridgeStatus, NodePreferences } from '../types'

const defaultPreferences: NodePreferences = {
  executablePath: '',
  rpcEndpoint: 'http://127.0.0.1:8080/api/v1',
  network: 'private-testnet',
  launchOnStartup: false,
}

export async function getDesktopBridgeStatus(): Promise<DesktopBridgeStatus> {
  try {
    return await invoke<DesktopBridgeStatus>('get_desktop_status')
  } catch {
    return {
      appVersion: 'web-preview',
      platform: 'browser',
      nodeConfigured: false,
      nodeRunning: false,
      rpcReachable: false,
    }
  }
}

export function loadNodePreferences(): NodePreferences {
  const raw = localStorage.getItem('pulsedag.desktop.node-preferences')
  if (!raw) return defaultPreferences

  try {
    return { ...defaultPreferences, ...JSON.parse(raw) } as NodePreferences
  } catch {
    return defaultPreferences
  }
}

export function saveNodePreferences(preferences: NodePreferences): void {
  localStorage.setItem('pulsedag.desktop.node-preferences', JSON.stringify(preferences))
}
