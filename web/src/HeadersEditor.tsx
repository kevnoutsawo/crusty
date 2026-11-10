import type { Header } from './core'
import { emptyHeader } from './emptyHeader'

interface Props {
  headers: Header[]
  onChange: (headers: Header[]) => void
}

export function HeadersEditor({ headers, onChange }: Props) {
  const update = (i: number, patch: Partial<Header>) =>
    onChange(headers.map((h, j) => (j === i ? { ...h, ...patch } : h)))
  const remove = (i: number) => {
    const rest = headers.filter((_, j) => j !== i)
    onChange(rest.length ? rest : [emptyHeader()])
  }

  return (
    <div className="headers">
      {headers.map((h, i) => (
        <div className="header-row" key={i}>
          <input
            type="checkbox"
            checked={h.enabled}
            onChange={(e) => update(i, { enabled: e.target.checked })}
            aria-label={`Send header ${i + 1}`}
          />
          <input
            className="field"
            value={h.key}
            onChange={(e) => update(i, { key: e.target.value })}
            placeholder="Header"
            aria-label={`Header ${i + 1} name`}
            spellCheck={false}
          />
          <input
            className="field"
            value={h.value}
            onChange={(e) => update(i, { value: e.target.value })}
            placeholder="Value"
            aria-label={`Header ${i + 1} value`}
            spellCheck={false}
          />
          <button
            type="button"
            className="icon-button"
            onClick={() => remove(i)}
            aria-label={`Remove header ${i + 1}`}
          >
            ×
          </button>
        </div>
      ))}
      <button type="button" className="link-button" onClick={() => onChange([...headers, emptyHeader()])}>
        + Add header
      </button>
    </div>
  )
}
