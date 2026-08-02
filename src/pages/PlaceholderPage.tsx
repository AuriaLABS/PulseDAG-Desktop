type PlaceholderPageProps = {
  eyebrow: string
  title: string
  description: string
  items: string[]
}

export function PlaceholderPage({ eyebrow, title, description, items }: PlaceholderPageProps) {
  return (
    <section className="panel placeholder-panel">
      <span className="eyebrow">{eyebrow}</span>
      <h2>{title}</h2>
      <p>{description}</p>
      <div className="placeholder-grid">
        {items.map((item, index) => <div key={item}><span>{String(index + 1).padStart(2, '0')}</span><strong>{item}</strong></div>)}
      </div>
    </section>
  )
}
