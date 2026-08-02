import { useEffect, useMemo, useState } from 'react'
import { Sidebar } from '../components/Sidebar'
import { getDesktopBridgeStatus, loadNodePreferences, saveNodePreferences } from '../lib/desktop'
import { NodePage } from '../pages/NodePage'
import { OverviewPage } from '../pages/OverviewPage'
import { PlaceholderPage } from '../pages/PlaceholderPage'
import { SettingsPage } from '../pages/SettingsPage'
import type { AppSection, DesktopBridgeStatus, NodePreferences } from '../types'

const sectionTitles: Record<AppSection, { eyebrow: string; title: string }> = {
  overview: { eyebrow: 'PulseDAG operator workspace', title: 'Overview' },
  node: { eyebrow: 'Local process supervision', title: 'Node' },
  network: { eyebrow: 'Peer and synchronization intelligence', title: 'Network' },
  dag: { eyebrow: 'Realtime consensus activity', title: 'Live DAG' },
  logs: { eyebrow: 'Local diagnostics', title: 'Logs' },
  settings: { eyebrow: 'Desktop preferences', title: 'Settings' },
}

export function App() {
  const [section, setSection] = useState<AppSection>('overview')
  const [theme, setTheme] = useState<'dark' | 'light'>(() => (localStorage.getItem('pulsedag.desktop.theme') === 'light' ? 'light' : 'dark'))
  const [bridge, setBridge] = useState<DesktopBridgeStatus | null>(null)
  const [preferences, setPreferences] = useState<NodePreferences>(() => loadNodePreferences())

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem('pulsedag.desktop.theme', theme)
  }, [theme])

  useEffect(() => {
    void getDesktopBridgeStatus().then(setBridge)
  }, [])

  const heading = sectionTitles[section]
  const content = useMemo(() => {
    if (section === 'overview') return <OverviewPage bridge={bridge} preferences={preferences} onOpenSettings={() => setSection('settings')} />
    if (section === 'node') return <NodePage preferences={preferences} onOpenSettings={() => setSection('settings')} />
    if (section === 'settings') {
      return <SettingsPage value={preferences} onSave={(next) => { saveNodePreferences(next); setPreferences(next) }} />
    }
    if (section === 'network') {
      return <PlaceholderPage eyebrow="Network observability" title="Peers and synchronization" description="This area will consume approved local status endpoints and present sync lag, peer health and connection history." items={['Peer table', 'Sync progress', 'Connection health', 'Network identity']} />
    }
    if (section === 'dag') {
      return <PlaceholderPage eyebrow="Consensus visualization" title="Live DAG workspace" description="The explorer DAG components will be adapted for local node data without exposing operator controls to the public web client." items={['Realtime block graph', 'Tip selection', 'Block inspector', 'Performance timeline']} />
    }
    return <PlaceholderPage eyebrow="Diagnostics" title="Structured local logs" description="Logs will be streamed from the supervised process, filtered locally and exportable as a support bundle." items={['Live stream', 'Severity filters', 'Search', 'Diagnostic export']} />
  }, [bridge, preferences, section])

  return (
    <div className="app-shell">
      <Sidebar active={section} onChange={setSection} />
      <main>
        <header className="topbar">
          <div><span className="eyebrow">{heading.eyebrow}</span><h1>{heading.title}</h1></div>
          <div className="topbar-actions">
            <span className="sync-pill warning"><i />Node offline</span>
            <button className="icon-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} aria-label="Toggle theme">{theme === 'dark' ? '☼' : '◐'}</button>
          </div>
        </header>
        {content}
      </main>
    </div>
  )
}
