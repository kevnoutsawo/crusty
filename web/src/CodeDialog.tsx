import { useEffect, useState } from 'react'
import { CopyButton } from './CopyButton'
import { Dialog } from './Dialog'
import { errorMessage, generateCode, languages, type Language, type WebRequest } from './core'

interface Props {
  request: WebRequest
  onClose: () => void
}

export function CodeDialog({ request, onClose }: Props) {
  const [langs, setLangs] = useState<Language[]>([])
  const [lang, setLang] = useState('curl')
  const [code, setCode] = useState('')
  const [error, setError] = useState<string>()

  useEffect(() => {
    languages().then(setLangs, (e) => setError(errorMessage(e)))
  }, [])

  useEffect(() => {
    let current = true
    generateCode(request, lang).then(
      (c) => {
        if (current) {
          setCode(c)
          setError(undefined)
        }
      },
      (e) => current && setError(errorMessage(e)),
    )
    return () => {
      current = false
    }
  }, [request, lang])

  return (
    <Dialog title="Generate code" onClose={onClose}>
      <div className="dialog-body">
        <div className="row">
          <label htmlFor="code-language">Language</label>
          <select
            id="code-language"
            className="field"
            value={lang}
            onChange={(e) => setLang(e.target.value)}
          >
            {langs.map((l) => (
              <option key={l.id} value={l.id}>
                {l.label}
              </option>
            ))}
          </select>
          <span className="spacer" />
          <CopyButton text={code} label="Copy code" />
        </div>
        {error ? (
          <p role="alert" className="error">
            {error}
          </p>
        ) : (
          <pre className="code" aria-label="Generated code">
            {code}
          </pre>
        )}
      </div>
    </Dialog>
  )
}
