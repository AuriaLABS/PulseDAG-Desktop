import type { AppSection } from '../types'

const sections: Array<{ id: AppSection; icon: string; label: string }> = [
  { id: 'overview', icon: '⌁', label: 'Overview' },
  { id: 'node', icon: '◉', label: 'Node' },
  { id: 'mining', icon: '⚒', label: 'Mining' },
  { id: 'network', icon: '◎', label: 'Network' },
  { id: 'dag', icon: '◇', label: 'Live DAG' },
  { id: 'logs', icon: '≡', label: 'Logs' },
  { id: 'settings', icon: '⚙', label: 'Settings' },
]

type SidebarProps = {
  active: AppSection
  onChange: (section: AppSection) => void
}

export function Sidebar({ active, onChange }: SidebarProps) {
  return (
    <aside className="sidebar">
      <button className="brand" onClick={() => onChange('overview')} aria-label="PulseDAG Desktop home">
        <span className="brand-mark" aria-hidden="true"><i /><i /><i /></span>
        <span><strong>PulseDAG</strong><small>DESKTOP</small></span>
      </button>

      <nav aria-label="Desktop navigation">
        {sections.map((section) => (
          <button
            key={section.id}
            className={active === section.id ? 'active' : ''}
            onClick={() => onChange(section.id)}
          >
            <span aria-hidden="true">{section.icon}</span>
            {section.label}
          </button>
        ))}
      </nav>

      <div className="sidebar-footer">
        <div className="network-indicator warning"><i />Node offline</div>
        <small>Local control plane</small>
        <small>Version 0.1.0</small>
      </div>
    </aside>
  )
}
