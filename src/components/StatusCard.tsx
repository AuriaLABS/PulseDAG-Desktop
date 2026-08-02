type StatusCardProps = {
  label: string
  value: string
  detail: string
  tone?: 'neutral' | 'success' | 'warning'
}

export function StatusCard({ label, value, detail, tone = 'neutral' }: StatusCardProps) {
  return (
    <article className={`metric-card tone-${tone}`}>
      <div className="metric-card-topline"><span>{label}</span><i /></div>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  )
}
