export interface RailItem {
  id: string
  label: string
}

export function Rail({
  items,
  active,
  onSelect,
}: {
  items: RailItem[]
  active: string
  onSelect: (id: string) => void
}) {
  return (
    <nav className="k-rail">
      <div className="k-rail-brand">
        <img src="/icon.svg" alt="" width={24} height={24} />
        <span>Abakus</span>
      </div>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className={`k-rail-item${item.id === active ? ' is-active' : ''}`}
          onClick={() => onSelect(item.id)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  )
}
