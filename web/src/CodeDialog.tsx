import { useEffect, useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { CopyButton } from './CopyButton'
import { errorMessage, generateCode, languages, type Language, type WebRequest } from './core'

interface Props {
  open: boolean
  request: WebRequest
  onClose: () => void
}

export function CodeDialog({ open, request, onClose }: Props) {
  const [langs, setLangs] = useState<Language[]>([])
  const [lang, setLang] = useState('curl')
  const [code, setCode] = useState('')
  const [error, setError] = useState<string>()

  useEffect(() => {
    languages().then(setLangs, (e) => setError(errorMessage(e)))
  }, [])

  useEffect(() => {
    if (!open) return
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
  }, [open, request, lang])

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Generate code</DialogTitle>
          <DialogDescription>A snippet that sends the current request.</DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-2">
          <Label id="code-language-label">Language</Label>
          <Select
            items={langs.map((l) => ({ value: l.id, label: l.label }))}
            value={lang}
            onValueChange={(v) => v && setLang(v)}
          >
            <SelectTrigger size="sm" aria-labelledby="code-language-label" className="min-w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {langs.map((l) => (
                <SelectItem key={l.id} value={l.id}>
                  {l.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <span className="flex-1" />
          <CopyButton text={code} label="Copy code" />
        </div>
        {error ? (
          <p role="alert" className="text-destructive">
            {error}
          </p>
        ) : (
          <ScrollArea className="max-h-[50vh] rounded-lg border bg-background">
            <pre className="px-4 py-3 font-mono leading-relaxed" aria-label="Generated code">
              {code}
            </pre>
            <ScrollBar orientation="horizontal" />
          </ScrollArea>
        )}
      </DialogContent>
    </Dialog>
  )
}
