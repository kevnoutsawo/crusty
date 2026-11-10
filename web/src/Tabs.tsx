import { useId, type ReactNode } from 'react'

interface Props<T extends string> {
  label: string
  tabs: readonly { id: T; label: string }[]
  active: T
  onChange: (id: T) => void
  children: ReactNode
}

// An ARIA tab list with arrow-key navigation; only the active panel renders.
export function Tabs<T extends string>({ label, tabs, active, onChange, children }: Props<T>) {
  const base = useId()
  const move = (from: number, by: number) => {
    const next = tabs[(from + by + tabs.length) % tabs.length]
    onChange(next.id)
    document.getElementById(`${base}-tab-${next.id}`)?.focus()
  }

  return (
    <div className="tabs">
      <div role="tablist" aria-label={label} className="tablist">
        {tabs.map((t, i) => (
          <button
            key={t.id}
            id={`${base}-tab-${t.id}`}
            type="button"
            role="tab"
            aria-selected={t.id === active}
            aria-controls={`${base}-panel`}
            tabIndex={t.id === active ? 0 : -1}
            className="tab"
            onClick={() => onChange(t.id)}
            onKeyDown={(e) => {
              if (e.key === 'ArrowRight') move(i, 1)
              if (e.key === 'ArrowLeft') move(i, -1)
            }}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`${base}-panel`}
        aria-labelledby={`${base}-tab-${active}`}
        className="tabpanel"
      >
        {children}
      </div>
    </div>
  )
}
