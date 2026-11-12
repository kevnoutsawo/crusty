import { useState, type FormEvent } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { errorMessage, importCurl, type WebRequest } from './core'

interface Props {
  open: boolean
  onImport: (request: WebRequest) => void
  onClose: () => void
}

export function CurlDialog({ open, onImport, onClose }: Props) {
  const [command, setCommand] = useState('')
  const [error, setError] = useState<string>()

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    try {
      onImport(await importCurl(command))
      setCommand('')
      setError(undefined)
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <form onSubmit={submit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Import cURL command</DialogTitle>
            <DialogDescription>The method, URL, headers and body fill the request.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="curl-command">Paste a cURL command</Label>
            <Textarea
              id="curl-command"
              className="min-h-32 font-mono"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder={`curl -X POST https://api.example.com/users -H 'Content-Type: application/json' -d '{"name":"Alice"}'`}
              spellCheck={false}
              autoFocus
            />
            {error && (
              <p role="alert" className="text-destructive">
                {error}
              </p>
            )}
          </div>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" />}>Cancel</DialogClose>
            <Button type="submit" disabled={!command.trim()}>
              Import
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
