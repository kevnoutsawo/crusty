import { useEffect, useState } from 'react'

export function CopyButton({ text, label }: { text: string; label: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle')

  useEffect(() => {
    if (state === 'idle') return
    const t = setTimeout(() => setState('idle'), 1500)
    return () => clearTimeout(t)
  }, [state])

  const copy = () =>
    navigator.clipboard.writeText(text).then(
      () => setState('copied'),
      () => setState('failed'),
    )

  return (
    <button type="button" className="button small" onClick={copy} disabled={!text}>
      <span aria-live="polite">
        {state === 'copied' ? 'Copied' : state === 'failed' ? "Couldn't copy" : label}
      </span>
    </button>
  )
}
