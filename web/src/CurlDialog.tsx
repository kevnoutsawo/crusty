import { useState, type FormEvent } from 'react'
import { Dialog } from './Dialog'
import { errorMessage, importCurl, type WebRequest } from './core'

interface Props {
  onImport: (request: WebRequest) => void
  onClose: () => void
}

export function CurlDialog({ onImport, onClose }: Props) {
  const [command, setCommand] = useState('')
  const [error, setError] = useState<string>()

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    try {
      onImport(await importCurl(command))
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <Dialog title="Import cURL command" onClose={onClose}>
      <form onSubmit={submit} className="dialog-body">
        <label htmlFor="curl-command">Paste a cURL command</label>
        <textarea
          id="curl-command"
          className="field mono"
          rows={6}
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          placeholder="curl -X POST https://api.example.com/users -H 'Content-Type: application/json' -d '{&quot;name&quot;:&quot;Alice&quot;}'"
          spellCheck={false}
        />
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" className="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="button primary" disabled={!command.trim()}>
            Import
          </button>
        </div>
      </form>
    </Dialog>
  )
}
