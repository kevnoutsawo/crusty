import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import * as core from './core'

// The real module loads the generated WebAssembly; the UI tests stand in for it.
vi.mock('./core', async () => {
  const types = await vi.importActual<typeof import('./core/types')>('./core/types')
  return {
    ...types,
    sendRequest: vi.fn(),
    importCurl: vi.fn(),
    generateCode: vi.fn(),
    languages: vi.fn(),
    errorMessage: (e: unknown) => (typeof e === 'string' ? e : String(e)),
  }
})

const mocked = vi.mocked(core)

// Base UI's Select is a combobox with a popup listbox, not a native <select>.
async function choose(user: ReturnType<typeof userEvent.setup>, name: string, option: string) {
  await user.click(screen.getByRole('combobox', { name }))
  await user.click(await screen.findByRole('option', { name: option }))
}

const okResponse: core.WebResponse = {
  status: 200,
  statusText: 'OK',
  headers: [['content-type', 'application/json']],
  body: '{\n  "id": 1\n}',
  bodyIsJson: true,
  bodySize: 2048,
  totalMs: 12.4,
}

describe('App', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    mocked.languages.mockResolvedValue([
      { id: 'curl', label: 'cURL' },
      { id: 'python', label: 'Python (requests)' },
    ])
  })

  it('sends the edited request and shows the response', async () => {
    const user = userEvent.setup()
    mocked.sendRequest.mockResolvedValue(okResponse)
    render(<App />)

    await choose(user, 'Method', 'POST')
    await user.type(screen.getByLabelText('Header 1 name'), 'Accept')
    await user.type(screen.getByLabelText('Header 1 value'), 'application/json')
    await user.type(screen.getByLabelText('URL'), 'https://api.example.com/users{Enter}')

    expect(mocked.sendRequest).toHaveBeenCalledWith({
      method: 'POST',
      url: 'https://api.example.com/users',
      headers: [{ key: 'Accept', value: 'application/json', enabled: true }],
      body: '',
    })
    const response = await screen.findByRole('region', { name: 'Response' })
    expect(within(response).getByText('200 OK')).toBeInTheDocument()
    expect(within(response).getByText('12 ms')).toBeInTheDocument()
    expect(within(response).getByText('2.0 KB')).toBeInTheDocument()
    expect(within(response).getByLabelText('Response body')).toHaveTextContent('"id": 1')

    await user.click(within(response).getByRole('tab', { name: 'Headers' }))
    expect(within(response).getByRole('rowheader', { name: 'content-type' })).toBeInTheDocument()
  })

  it('disables Send until there is a URL', async () => {
    const user = userEvent.setup()
    render(<App />)
    const send = screen.getByRole('button', { name: 'Send' })
    expect(send).toBeDisabled()
    await user.type(screen.getByLabelText('URL'), '   ')
    expect(send).toBeDisabled()
    await user.type(screen.getByLabelText('URL'), 'x')
    expect(send).toBeEnabled()
  })

  it('warns that a GET body will not be sent until the method changes', async () => {
    const user = userEvent.setup()
    render(<App />)

    await user.click(screen.getByRole('tab', { name: 'Body' }))
    await user.type(screen.getByLabelText('Request body'), 'hello')
    expect(screen.getByText(/Browsers can't send a body with GET/)).toBeInTheDocument()

    await choose(user, 'Method', 'POST')
    expect(screen.queryByText(/Browsers can't send a body/)).not.toBeInTheDocument()
  })

  it('shows the core error message when a request fails', async () => {
    const user = userEvent.setup()
    mocked.sendRequest.mockRejectedValue('Connection refused. Is the server running?')
    render(<App />)

    await user.type(screen.getByLabelText('URL'), 'http://localhost:1{Enter}')

    expect(await screen.findByRole('alert')).toHaveTextContent('Connection refused. Is the server running?')
  })

  it('fills the request from an imported cURL command', async () => {
    const user = userEvent.setup()
    mocked.importCurl.mockResolvedValue({
      method: 'PUT',
      url: 'https://api.example.com/users/1',
      headers: [{ key: 'Content-Type', value: 'application/json', enabled: true }],
      body: '{"name":"Bob"}',
    })
    render(<App />)

    await user.click(screen.getByRole('button', { name: 'Import cURL' }))
    const dialog = screen.getByRole('dialog', { name: 'Import cURL command' })
    await user.type(within(dialog).getByLabelText('Paste a cURL command'), 'curl -X PUT …')
    await user.click(within(dialog).getByRole('button', { name: 'Import' }))

    expect(mocked.importCurl).toHaveBeenCalledWith('curl -X PUT …')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(screen.getByRole('combobox', { name: 'Method' })).toHaveTextContent('PUT')
    expect(screen.getByLabelText('URL')).toHaveValue('https://api.example.com/users/1')
    expect(screen.getByRole('tab', { name: 'Body' })).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByLabelText('Request body')).toHaveValue('{"name":"Bob"}')
  })

  it('keeps the cURL dialog open and shows why an import failed', async () => {
    const user = userEvent.setup()
    mocked.importCurl.mockRejectedValue('no URL found in curl command')
    render(<App />)

    await user.click(screen.getByRole('button', { name: 'Import cURL' }))
    const dialog = screen.getByRole('dialog')
    await user.type(within(dialog).getByLabelText('Paste a cURL command'), 'curl')
    await user.click(within(dialog).getByRole('button', { name: 'Import' }))

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('no URL found')
  })

  it('generates code for the current request in the chosen language', async () => {
    const user = userEvent.setup()
    mocked.generateCode.mockImplementation(async (_req, lang) => `code for ${lang}`)
    render(<App />)

    await user.type(screen.getByLabelText('URL'), 'https://api.example.com')
    await user.click(screen.getByRole('button', { name: 'Generate code' }))
    const dialog = screen.getByRole('dialog', { name: 'Generate code' })

    expect(await within(dialog).findByText('code for curl')).toBeInTheDocument()
    await choose(user, 'Language', 'Python (requests)')
    expect(await within(dialog).findByText('code for python')).toBeInTheDocument()
    expect(mocked.generateCode).toHaveBeenLastCalledWith(
      expect.objectContaining({ url: 'https://api.example.com' }),
      'python',
    )

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
