export type AppSection = 'overview' | 'node' | 'network' | 'dag' | 'logs' | 'settings'

export type DesktopBridgeStatus = {
  appVersion: string
  platform: string
  nodeConfigured: boolean
  nodeRunning: boolean
  rpcReachable: boolean
}

export type NodePreferences = {
  executablePath: string
  rpcEndpoint: string
  network: 'private-testnet' | 'devnet' | 'testnet' | 'mainnet'
  launchOnStartup: boolean
}
