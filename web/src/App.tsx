import { CircleAlertIcon, CodeIcon, FileInputIcon, Loader2Icon, SendIcon } from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'
import { CodeDialog } from './CodeDialog'
import { CurlDialog } from './CurlDialog'
import { METHODS, errorMessage, sendRequest, type WebRequest, type WebResponse } from './core'
import { emptyHeader } from './emptyHeader'
import { HeadersEditor } from './HeadersEditor'
import { ResponseView } from './ResponseView'

type Result =
  | { kind: 'idle' }
  | { kind: 'sending' }
  | { kind: 'done'; response: WebResponse }
  | { kind: 'error'; message: string }

const METHOD_COLOR: Record<string, string> = {
  GET: 'text-method-get',
  POST: 'text-method-post',
  DELETE: 'text-method-delete',
}

const newRequest = (): WebRequest => ({ method: 'GET', url: '', headers: [emptyHeader()], body: '' })

export default function App() {
  const [request, setRequest] = useState<WebRequest>(newRequest)
  const [requestTab, setRequestTab] = useState('headers')
  const [result, setResult] = useState<Result>({ kind: 'idle' })
  const [dialog, setDialog] = useState<'curl' | 'code'>()

  const edit = (patch: Partial<WebRequest>) => setRequest((r) => ({ ...r, ...patch }))
  const sending = result.kind === 'sending'
  const hasUrl = request.url.trim() !== ''
  const bodyNotSent = request.body.trim() !== '' && ['GET', 'HEAD'].includes(request.method)
  const closeDialog = () => setDialog(undefined)

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
    closeDialog()
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-11 items-center gap-2 border-b bg-card px-4">
        <h1 className="text-[15px] font-semibold tracking-tight">Crusty</h1>
        <span className="flex-1" />
        <Button variant="outline" size="sm" onClick={() => setDialog('curl')}>
          <FileInputIcon data-icon="inline-start" />
          Import cURL
        </Button>
        <Button variant="outline" size="sm" onClick={() => setDialog('code')} disabled={!hasUrl}>
          <CodeIcon data-icon="inline-start" />
          Generate code
        </Button>
      </header>

      <form
        className="flex flex-wrap gap-2 border-b px-4 py-3 sm:flex-nowrap"
        onSubmit={send}
        aria-label="Request URL"
      >
        <Select value={request.method} onValueChange={(method) => method && edit({ method })}>
          <SelectTrigger
            aria-label="Method"
            className={cn('w-28 font-mono font-semibold', METHOD_COLOR[request.method] ?? 'text-method-other')}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {METHODS.map((m) => (
              <SelectItem key={m} value={m} className="font-mono">
                {m}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Input
          aria-label="URL"
          className="order-first basis-full font-mono sm:order-none sm:basis-auto"
          type="text"
          inputMode="url"
          value={request.url}
          onChange={(e) => edit({ url: e.target.value })}
          placeholder="https://api.example.com/users"
          autoFocus
          spellCheck={false}
          autoComplete="off"
        />
        <Button type="submit" disabled={!hasUrl || sending} className="min-w-20">
          {sending ? (
            <Loader2Icon data-icon="inline-start" className="animate-spin" />
          ) : (
            <SendIcon data-icon="inline-start" />
          )}
          {sending ? 'Sending…' : 'Send'}
        </Button>
      </form>

      <main className="grid min-h-0 flex-1 grid-rows-[auto_1fr] md:grid-cols-[minmax(320px,2fr)_3fr] md:grid-rows-1">
        <section className="flex min-h-0 flex-col border-b md:border-r md:border-b-0" aria-label="Request">
          <Tabs value={requestTab} onValueChange={setRequestTab} className="min-h-0 flex-1 gap-0 pt-2">
            <TabsList variant="line" className="mx-3" aria-label="Request">
              <TabsTrigger value="headers">Headers</TabsTrigger>
              <TabsTrigger value="body">Body</TabsTrigger>
            </TabsList>
            <TabsContent value="headers" className="min-h-0 overflow-auto border-t">
              <HeadersEditor headers={request.headers} onChange={(headers) => edit({ headers })} />
            </TabsContent>
            <TabsContent value="body" className="flex min-h-0 flex-col gap-2 border-t p-3">
              <Textarea
                aria-label="Request body"
                className="min-h-40 flex-1 resize-none font-mono"
                value={request.body}
                onChange={(e) => edit({ body: e.target.value })}
                placeholder='{"name": "Alice"}'
                spellCheck={false}
                aria-describedby={bodyNotSent ? 'body-note' : undefined}
              />
              {bodyNotSent && (
                <p id="body-note" className="text-warning">
                  Browsers can't send a body with {request.method}. Choose another method or clear the body.
                </p>
              )}
            </TabsContent>
          </Tabs>
        </section>

        <div className="flex min-h-0 min-w-0 flex-col">
          {result.kind === 'idle' && (
            <p className="m-4 text-muted-foreground">
              Enter a URL and press Enter or Send. The response appears here.
            </p>
          )}
          {result.kind === 'sending' && (
            <p className="m-4 text-muted-foreground" role="status">
              Sending request…
            </p>
          )}
          {result.kind === 'error' && (
            <div
              role="alert"
              className="m-4 flex gap-2.5 rounded-lg border border-destructive/40 bg-destructive/5 px-3.5 py-3 text-destructive"
            >
              <CircleAlertIcon className="mt-0.5 size-4 shrink-0" aria-hidden />
              <div className="min-w-0">
                <strong className="font-semibold">Request failed</strong>
                <p className="mt-0.5 break-words">{result.message}</p>
              </div>
            </div>
          )}
          {result.kind === 'done' && <ResponseView response={result.response} />}
        </div>
      </main>

      <CurlDialog open={dialog === 'curl'} onImport={imported} onClose={closeDialog} />
      <CodeDialog open={dialog === 'code'} request={request} onClose={closeDialog} />
    </div>
  )
}
