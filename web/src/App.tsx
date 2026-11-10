import { useState, type FormEvent } from 'react'
import { CodeDialog } from './CodeDialog'
import { CurlDialog } from './CurlDialog'
import { emptyHeader } from './emptyHeader'
import { HeadersEditor } from './HeadersEditor'
import { ResponseView } from './ResponseView'
import { Tabs } from './Tabs'
import { METHODS, errorMessage, sendRequest, type WebRequest, type WebResponse } from './core'

type Result =
  | { kind: 'idle' }
  | { kind: 'sending' }
  | { kind: 'done'; response: WebResponse }
  | { kind: 'error'; message: string }

const REQUEST_TABS = [
  { id: 'headers', label: 'Headers' },
  { id: 'body', label: 'Body' },
] as const

const newRequest = (): WebRequest => ({ method: 'GET', url: '', headers: [emptyHeader()], body: '' })

export default function App() {
  const [request, setRequest] = useState<WebRequest>(newRequest)
  const [requestTab, setRequestTab] = useState<(typeof REQUEST_TABS)[number]['id']>('headers')
  const [result, setResult] = useState<Result>({ kind: 'idle' })
  const [dialog, setDialog] = useState<'curl' | 'code'>()

  const edit = (patch: Partial<WebRequest>) => setRequest((r) => ({ ...r, ...patch }))
  const sending = result.kind === 'sending'
  const hasUrl = request.url.trim() !== ''
  const bodyNotSent = request.body.trim() !== '' && ['GET', 'HEAD'].includes(request.method)

  const send = async (e: FormEvent) => {
    e.preventDefault()
    if (!hasUrl || sending) return
    setResult({ kind: 'sending' })
    try {
      setResult({ kind: 'done', response: await sendRequest(request) })
    } catch (err) {
      setResult({ kind: 'error', message: errorMessage(err) })
    }
  }

  const imported = (r: WebRequest) => {
    setRequest({ ...r, headers: r.headers.length ? r.headers : [emptyHeader()] })
    if (r.body) setRequestTab('body')
    setDialog(undefined)
  }

  return (
    <div className="app">
      <header className="topbar">
        <h1 className="brand">Crusty</h1>
        <span className="spacer" />
        <button type="button" className="button" onClick={() => setDialog('curl')}>
          Import cURL
        </button>
        <button type="button" className="button" onClick={() => setDialog('code')} disabled={!hasUrl}>
          Generate code
        </button>
      </header>

      <form className="request-bar" onSubmit={send}>
        <label className="visually-hidden" htmlFor="method">
          Method
        </label>
        <select
          id="method"
          className={`field method method-${request.method.toLowerCase()}`}
          value={request.method}
          onChange={(e) => edit({ method: e.target.value })}
        >
          {METHODS.map((m) => (
            <option key={m}>{m}</option>
          ))}
        </select>
        <label className="visually-hidden" htmlFor="url">
          URL
        </label>
        <input
          id="url"
          className="field url"
          type="text"
          inputMode="url"
          value={request.url}
          onChange={(e) => edit({ url: e.target.value })}
          placeholder="https://api.example.com/users"
          autoFocus
          spellCheck={false}
          autoComplete="off"
        />
        <button type="submit" className="button primary" disabled={!hasUrl || sending}>
          {sending ? 'Sending…' : 'Send'}
        </button>
      </form>

      <main className="panes">
        <section className="pane" aria-label="Request">
          <Tabs label="Request" tabs={REQUEST_TABS} active={requestTab} onChange={setRequestTab}>
            {requestTab === 'headers' ? (
              <HeadersEditor headers={request.headers} onChange={(headers) => edit({ headers })} />
            ) : (
              <>
                <label className="visually-hidden" htmlFor="body">
                  Request body
                </label>
                <textarea
                  id="body"
                  className="field mono body-editor"
                  value={request.body}
                  onChange={(e) => edit({ body: e.target.value })}
                  placeholder='{"name": "Alice"}'
                  spellCheck={false}
                  aria-describedby={bodyNotSent ? 'body-note' : undefined}
                />
                {bodyNotSent && (
                  <p id="body-note" className="note">
                    Browsers can't send a body with {request.method}. Choose another method or clear
                    the body.
                  </p>
                )}
              </>
            )}
          </Tabs>
        </section>

        <div className="pane">
          {result.kind === 'idle' && (
            <p className="hint">Enter a URL and press Enter or Send. The response appears here.</p>
          )}
          {result.kind === 'sending' && (
            <p className="hint" role="status">
              Sending request…
            </p>
          )}
          {result.kind === 'error' && (
            <div role="alert" className="error-box">
              <strong>Request failed</strong>
              <p>{result.message}</p>
            </div>
          )}
          {result.kind === 'done' && <ResponseView response={result.response} />}
        </div>
      </main>

      {dialog === 'curl' && <CurlDialog onImport={imported} onClose={() => setDialog(undefined)} />}
      {dialog === 'code' && <CodeDialog request={request} onClose={() => setDialog(undefined)} />}
    </div>
  )
}
