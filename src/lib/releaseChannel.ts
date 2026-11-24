/**
 * A testnet preview is a separate installed app and must not reuse the
 * persisted preferences of the stable/mainnet Desktop.
 *
 * This is presentation and local-storage isolation only, never Core GO.
 */
export const isTestnetPreviewBuild =
  import.meta.env.VITE_PULSEDAG_DESKTOP_CHANNEL === 'testnet'

export const nodePreferencesStorageKey = isTestnetPreviewBuild
  ? 'pulsedag.desktop.testnet.node-preferences'
  : 'pulsedag.desktop.node-preferences'

export const themeStorageKey = isTestnetPreviewBuild
  ? 'pulsedag.desktop.testnet.theme'
  : 'pulsedag.desktop.theme'
