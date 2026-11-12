import { PlusIcon, XIcon } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
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
    <div className="flex flex-col gap-1.5 p-3">
      {headers.map((h, i) => (
        <div className="grid grid-cols-[auto_1fr_1.5fr_auto] items-center gap-1.5" key={i}>
          <Checkbox
            className="mx-1"
            checked={h.enabled}
            onCheckedChange={(enabled) => update(i, { enabled })}
            aria-label={`Send header ${i + 1}`}
          />
          <Input
            className="font-mono"
            value={h.key}
            onChange={(e) => update(i, { key: e.target.value })}
            placeholder="Header"
            aria-label={`Header ${i + 1} name`}
            spellCheck={false}
          />
          <Input
            className="font-mono"
            value={h.value}
            onChange={(e) => update(i, { value: e.target.value })}
            placeholder="Value"
            aria-label={`Header ${i + 1} value`}
            spellCheck={false}
          />
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => remove(i)}
            aria-label={`Remove header ${i + 1}`}
          >
            <XIcon />
          </Button>
        </div>
      ))}
      <Button
        variant="ghost"
        size="sm"
        className="self-start text-primary hover:text-primary"
        onClick={() => onChange([...headers, emptyHeader()])}
      >
        <PlusIcon data-icon="inline-start" />
        Add header
      </Button>
    </div>
  )
}
