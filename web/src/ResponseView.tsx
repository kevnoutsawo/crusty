import { useState } from 'react'
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area'
import { Table, TableBody, TableCell, TableHead, TableRow } from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { cn } from '@/lib/utils'
import { CopyButton } from './CopyButton'
import type { WebResponse } from './core'

const STATUS_COLOR: Record<number, string> = {
  1: 'text-primary',
  2: 'text-success',
  3: 'text-warning',
  4: 'text-destructive',
  5: 'text-destructive',
}

export function ResponseView({ response }: { response: WebResponse }) {
  const [tab, setTab] = useState('body')

  return (
    <section className="flex min-h-0 flex-1 flex-col" aria-label="Response">
      <div className="flex items-center gap-3 px-4 pt-3 pb-1 text-muted-foreground tabular-nums">
        <strong className={cn('font-semibold', STATUS_COLOR[Math.floor(response.status / 100)])}>
          {response.status} {response.statusText}
        </strong>
        <span>{Math.round(response.totalMs)} ms</span>
        <span>{formatSize(response.bodySize)}</span>
        <span className="flex-1" />
        <CopyButton text={response.body} label="Copy body" />
      </div>
      <Tabs value={tab} onValueChange={setTab} className="min-h-0 flex-1 gap-0">
        <TabsList variant="line" className="mx-3" aria-label="Response">
          <TabsTrigger value="body">Body</TabsTrigger>
          <TabsTrigger value="headers">Headers</TabsTrigger>
        </TabsList>
        <TabsContent value="body" className="min-h-0 border-t">
          <ScrollArea className="h-full">
            <pre className="px-4 py-3 font-mono leading-relaxed" aria-label="Response body">
              {response.body}
            </pre>
            <ScrollBar orientation="horizontal" />
          </ScrollArea>
        </TabsContent>
        <TabsContent value="headers" className="min-h-0 overflow-auto border-t px-2">
          <Table className="font-mono text-[12.5px]">
            <TableBody>
              {response.headers.map(([k, v]) => (
                <TableRow key={k}>
                  <TableHead scope="row" className="h-auto py-1.5 align-top font-normal text-primary">
                    {k}
                  </TableHead>
                  <TableCell className="py-1.5 break-all whitespace-normal">{v}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TabsContent>
      </Tabs>
    </section>
  )
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
