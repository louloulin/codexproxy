import { useMemo } from "react"
import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { cn } from "@/lib/utils"

type LogDetail = {
  id: number
  ts: number
  provider_id: string
  client_model: string
  upstream_model: string
  endpoint: string
  status_code: number
  duration_ms: number
  prompt_tokens: number | null
  completion_tokens: number | null
  total_tokens: number | null
  stream: boolean
  error_code: string | null
  error_snippet: string | null
  request_body: string | null
  response_body: string | null
  cached_tokens?: number | null
  tool_call_count?: number | null
}

function safeParse(text: string | null): Record<string, unknown> | null {
  if (!text) return null
  try {
    const v = JSON.parse(text)
    return typeof v === "object" && v !== null ? (v as Record<string, unknown>) : null
  } catch {
    return null
  }
}

function summarizeContent(content: unknown): string {
  if (content == null) return ""
  if (typeof content === "string") return content
  if (Array.isArray(content)) {
    return content
      .map((part) => {
        if (typeof part === "string") return part
        if (typeof part === "object" && part !== null) {
          const p = part as Record<string, unknown>
          if (typeof p.text === "string") return p.text
          if (p.type === "image_url" || p.type === "image") return "[image]"
        }
        return JSON.stringify(part)
      })
      .join("\n")
  }
  return JSON.stringify(content, null, 2)
}

interface StructuredDetailProps {
  detail: LogDetail
  className?: string
}

export function StructuredDetail({ detail, className }: StructuredDetailProps) {
  const reqJson = useMemo(
    () => safeParse(detail.request_body),
    [detail.request_body]
  )
  const respJson = useMemo(
    () => safeParse(detail.response_body),
    [detail.response_body]
  )

  const messages: Array<Record<string, unknown>> = Array.isArray(reqJson?.messages)
    ? (reqJson.messages as Array<Record<string, unknown>>)
    : []
  const tools: Array<Record<string, unknown>> = Array.isArray(reqJson?.tools) && (reqJson.tools as unknown[]).length > 0
    ? (reqJson.tools as Array<Record<string, unknown>>)
    : []
  const choices: Array<Record<string, unknown>> = Array.isArray(respJson?.choices)
    ? (respJson.choices as Array<Record<string, unknown>>)
    : []

  const hasMessages = messages.length > 0
  const hasTools = tools.length > 0
  const hasChoices = choices.length > 0

  return (
    <div className={cn("space-y-4", className)}>
      <Card>
        <CardContent className="pt-4">
          <div className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
            <div>
              <span className="text-muted-foreground">Model:</span>{" "}
              <code className="text-xs">{detail.upstream_model}</code>
            </div>
            <div>
              <span className="text-muted-foreground">Duration:</span> {detail.duration_ms} ms
            </div>
            <div>
              <span className="text-muted-foreground">Tokens:</span>{" "}
              {detail.prompt_tokens ?? "—"} / {detail.completion_tokens ?? "—"} / {detail.total_tokens ?? "—"}
            </div>
            {detail.cached_tokens != null && detail.cached_tokens > 0 && (
              <div>
                <span className="text-muted-foreground">Cached:</span> {detail.cached_tokens}
              </div>
            )}
            {detail.tool_call_count != null && detail.tool_call_count > 0 && (
              <div>
                <span className="text-muted-foreground">Tools:</span> {detail.tool_call_count}
              </div>
            )}
          </div>
          {detail.error_code && (
            <div className="mt-3 p-2 bg-destructive/10 rounded text-sm">
              <span className="text-destructive font-medium">{detail.error_code}</span>
              {detail.error_snippet && <span>: {detail.error_snippet}</span>}
            </div>
          )}
        </CardContent>
      </Card>

      {(hasMessages || hasTools || hasChoices) && (
        <Tabs defaultValue="messages" className="w-full">
          <TabsList className="grid grid-cols-3 w-fit">
            {hasMessages && (
              <TabsTrigger value="messages">
                {"Messages (" + messages.length + ")"}
              </TabsTrigger>
            )}
            {hasTools && (
              <TabsTrigger value="tools">
                {"Tools (" + tools.length + ")"}
              </TabsTrigger>
            )}
            {hasChoices && (
              <TabsTrigger value="completion">
                {"Completion (" + choices.length + ")"}
              </TabsTrigger>
            )}
          </TabsList>

          {hasMessages && (
            <TabsContent value="messages" className="space-y-3">
              {messages.map((m, i) => {
                const msgContent = summarizeContent(m.content)
                return (
                  <Card key={i}>
                    <CardContent className="pt-3">
                      <div className="flex items-center gap-2 mb-2">
                        <Badge variant="secondary">{String(m.role ?? "?")}</Badge>
                        {typeof m.name === "string" && (
                          <span className="text-xs text-muted-foreground">name: {m.name}</span>
                        )}
                      </div>
                      <pre className="text-xs whitespace-pre-wrap break-words font-mono bg-muted/50 p-2 rounded max-h-48 overflow-auto">
                        {msgContent}
                      </pre>
                      {Array.isArray(m.tool_calls) && (
                        <div className="mt-2 pl-2 border-l-2 border-muted">
                          <span className="text-xs text-muted-foreground">tool_calls:</span>
                          {(m.tool_calls as Array<Record<string, unknown>>).map((tc, j) => {
                            const fn = tc.function as Record<string, unknown> | undefined
                            const fnName = fn?.name as string | undefined
                            return (
                              <div key={j} className="mt-1">
                                <Badge variant="outline" className="text-xs">
                                  {fnName ?? "unknown"}
                                </Badge>
                              </div>
                            )
                          })}
                        </div>
                      )}
                      {m.tool_call_id != null ? (
                        <div className="mt-2 text-xs text-muted-foreground">
                          tool_call_id: <code>{String(m.tool_call_id)}</code>
                        </div>
                      ) : null}
                    </CardContent>
                  </Card>
                )
              })}
            </TabsContent>
          )}

          {hasTools && (
            <TabsContent value="tools">
              <Card>
                <CardContent className="pt-3">
                  <div className="flex flex-wrap gap-2">
                    {tools.map((tool, i) => {
                      const fn = tool.function as Record<string, unknown> | undefined
                      const name = (fn?.name as string) ?? (tool.type as string) ?? "tool-" + i
                      const desc = fn?.description as string | undefined
                      return (
                        <div key={i} className="border rounded p-2 text-sm">
                          <Badge variant="outline">{name}</Badge>
                          {desc && <p className="text-xs text-muted-foreground mt-1">{desc}</p>}
                        </div>
                      )
                    })}
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          )}

          {hasChoices && (
            <TabsContent value="completion" className="space-y-3">
              {choices.map((c, i) => {
                const msg = c.message as Record<string, unknown> | undefined
                const finishReason = c.finish_reason as string | number | undefined
                return (
                  <Card key={i}>
                    <CardContent className="pt-3">
                      {finishReason && (
                        <div className="mb-2">
                          <Badge variant="secondary" className="text-xs">
                            {String(finishReason)}
                          </Badge>
                          {c.index !== undefined && (
                            <span className="text-xs text-muted-foreground ml-2">index: {String(c.index)}</span>
                          )}
                        </div>
                      )}
                      {msg && (
                        <>
                          <div className="flex items-center gap-2 mb-2">
                            <Badge variant="outline">{String(msg.role ?? "?")}</Badge>
                          </div>
                          <pre className="text-xs whitespace-pre-wrap break-words font-mono bg-muted/50 p-2 rounded max-h-48 overflow-auto">
                            {summarizeContent(msg.content)}
                          </pre>
                          {Array.isArray(msg.tool_calls) && (
                            <div className="mt-2 pl-2 border-l-2 border-muted">
                              <span className="text-xs text-muted-foreground">tool_calls:</span>
                              {(msg.tool_calls as Array<Record<string, unknown>>).map((tc, j) => {
                                const fn = tc.function as Record<string, unknown>
                                const fnName = fn?.name as string | undefined
                                const args = fn?.arguments
                                return (
                                  <div key={j} className="mt-1">
                                    <Badge variant="outline" className="text-xs">
                                      {fnName ?? "unknown"}
                                    </Badge>
                                    <pre className="text-xs bg-muted/30 p-1 rounded mt-1">
                                      {JSON.stringify(args, null, 2)}
                                    </pre>
                                  </div>
                                )
                              })}
                            </div>
                          )}
                        </>
                      )}
                    </CardContent>
                  </Card>
                )
              })}
            </TabsContent>
          )}
        </Tabs>
      )}

      {respJson?.usage && typeof respJson.usage === "object" ? (
        <Card>
          <CardContent className="pt-3">
            <div className="text-sm text-muted-foreground mb-2">Usage</div>
            <div className="grid grid-cols-4 gap-2 text-xs">
              {Object.entries(respJson.usage as Record<string, unknown>).map(([key, value]) => (
                <div key={key} className="bg-muted/50 p-2 rounded">
                  <span className="text-muted-foreground">{key}:</span>{" "}
                  <span className="font-mono">{typeof value === "number" ? value.toLocaleString() : String(value)}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      ) : null}
    </div>
  )
}
