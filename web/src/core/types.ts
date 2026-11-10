// The JSON shapes crates/crusty-wasm/src/dto.rs sends and receives.

// Browsers refuse TRACE and CONNECT in fetch, so the web client leaves them out.
export const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const
export type Method = (typeof METHODS)[number]

export interface Header {
  key: string
  value: string
  enabled: boolean
}

export interface WebRequest {
  method: string
  url: string
  headers: Header[]
  body: string
}

export interface WebResponse {
  status: number
  statusText: string
  headers: [string, string][]
  body: string
  bodyIsJson: boolean
  bodySize: number
  totalMs: number
}

export interface Language {
  id: string
  label: string
}
