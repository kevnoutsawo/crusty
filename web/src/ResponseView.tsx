import { useState } from 'react'
import { CopyButton } from './CopyButton'
import { Tabs } from './Tabs'
import type { WebResponse } from './core'

const TABS = [
  { id: 'body', label: 'Body' },
  { id: 'headers', label: 'Headers' },
] as const

export function ResponseView({ response }: { response: WebResponse }) {
  const [tab, setTab] = useState<(typeof TABS)[number]['id']>('body')

  return (
    <section className="response" aria-label="Response">
      <div className="status-line">
        <strong className={`status status-${Math.floor(response.status / 100)}xx`}>
          {response.status} {response.statusText}
        </strong>
        <span>{Math.round(response.totalMs)} ms</span>
        <span>{formatSize(response.bodySize)}</span>
        <span className="spacer" />
        <CopyButton text={response.body} label="Copy body" />
      </div>
      <Tabs label="Response" tabs={TABS} active={tab} onChange={setTab}>
        {tab === 'body' ? (
          <pre className="code body" aria-label="Response body">
            {response.body}
          </pre>
        ) : (
          <table className="header-table">
            <tbody>
              {response.headers.map(([k, v]) => (
                <tr key={k}>
                  <th scope="row">{k}</th>
                  <td>{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Tabs>
    </section>
  )
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
