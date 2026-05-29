import { useTranslation } from "react-i18next"
import { useQuery, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useState, useEffect, useRef } from "react"
import { RefreshCw, Download, Trash2, ChevronDown, ChevronUp, X } from "lucide-react"
import { cn } from "@/lib/utils"
import { api } from "@/lib/api"
import { useSearchParams } from "react-router-dom"
import { StructuredDetail } from "./StructuredDetail"
import { PageTour, type TourStep } from "@/components/PageTour"

// Logs PageTour steps
const LOGS_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='logs-filters']",
    title: "Filters",
    description: "Filter logs by provider, model, or status (all/ok/error). Use the time range selector on Dashboard for broader analysis.",
    placement: "bottom",
  },
  {
    target: "[data-tour='logs-actions']",
    title: "Actions",
    description: "Refresh to reload logs, Export CSV to download all visible logs, or Clear Old to remove historical data.",
    placement: "bottom",
  },
  {
    target: "[data-tour='logs-table']",
    title: "Logs Table",
    description: "View request details. Click the expand icon to see structured request/response bodies. Status codes show error status at a glance.",
    placement: "top",
  },
]

type StatusFilter = "all" | "ok" | "error"

const PAGE_SIZE = 25

type LogRow = {
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
}

export function LogsPage() {
  const { t } = useTranslation()
  const [searchParams] = useSearchParams()
  const [provider, setProvider] = useState<string>("__all__")
  const [model, setModel] = useState("")
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all")
  const [page, setPage] = useState(0)
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null)
  const [showClearModal, setShowClearModal] = useState(false)
  const initialHighlightApplied = useRef(false)

  // Handle URL highlight parameter (?highlight=<id>)
  useEffect(() => {
    if (initialHighlightApplied.current) return
    const highlight = searchParams.get("highlight")
    if (highlight) {
      const id = parseInt(highlight, 10)
      if (!isNaN(id)) {
        setSelectedLogId(id)
        initialHighlightApplied.current = true
      }
    }
  }, [searchParams])

  // Calculate status codes based on filter
  const statusMin = statusFilter === "ok" ? 200 : statusFilter === "error" ? 400 : undefined
  const statusMax = statusFilter === "error" ? 599 : undefined

  const { data, isLoading, refetch } = useQuery({
    queryKey: ["logs", provider, model, statusFilter, page],
    queryFn: () => api.logs.list({
      provider: provider === "__all__" ? undefined : provider,
      model: model || undefined,
      statusMin,
      statusMax,
      limit: PAGE_SIZE,
      offset: page * PAGE_SIZE,
    }),
  })

  const logs = data?.logs ?? []
  const total = logs.length // Estimate total from returned logs (backend doesn't provide total count)
  const totalPages = Math.ceil(total / PAGE_SIZE) || 1

  // Providers list for dropdown - convert Record to array
  const { data: providersData } = useQuery({
    queryKey: ["providers-list"],
    queryFn: () => api.providers.list(),
  })
  // Convert Record<string, Provider> to Provider[]
  const providers: Array<{id: string, display_name: string}> = providersData
    ? Object.values(providersData as Record<string, {id: string, display_name?: string}>).map(p => ({
        id: p.id,
        display_name: p.display_name || p.id
      }))
    : []

  // Export to CSV
  function exportCsv() {
    if (!logs.length) return
    const headers = ["Time", "Provider", "Client Model", "Upstream Model", "Endpoint", "Status", "Duration", "Tokens", "Error"]
    const rows = logs.map((log) => [
      new Date(log.ts).toISOString(),
      log.provider_id,
      log.client_model,
      log.upstream_model,
      log.endpoint,
      String(log.status_code),
      String(log.duration_ms),
      String(log.total_tokens ?? ""),
      log.error_code ?? "",
    ])
    const csv = [headers, ...rows].map((r) => r.map((c) => `"${c}"`).join(",")).join("\n")
    const blob = new Blob([csv], { type: "text/csv" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `logs-${new Date().toISOString().slice(0, 10)}.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <PageTour pageKey="logs" steps={LOGS_TOUR_STEPS}>
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("logs.title", "Logs")}</h1>
        <p className="text-muted-foreground">
          {t("logs.subtitle", "View and analyze API request logs")}
        </p>
      </div>

      {/* Filters */}
      <Card>
        <CardContent className="py-4">
          <div className="flex flex-wrap gap-4 items-end" data-tour="logs-filters">
            {/* Provider Filter */}
            <div className="w-48">
              <label className="text-sm font-medium mb-1 block">{t("logs.provider", "Provider")}</label>
              <Select value={provider} onValueChange={(v) => { setProvider(v); setPage(0) }}>
                <SelectTrigger>
                  <SelectValue placeholder={t("logs.allProviders", "All providers")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__all__">{t("logs.allProviders", "All providers")}</SelectItem>
                  {providers.map((p) => (
                    <SelectItem key={p.id} value={p.id}>{p.display_name || p.id}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Model Filter */}
            <div className="w-48">
              <label className="text-sm font-medium mb-1 block">{t("logs.model", "Model")}</label>
              <Input
                placeholder={t("logs.modelPlaceholder", "Filter by model...")}
                value={model}
                onChange={(e) => { setModel(e.target.value); setPage(0) }}
              />
            </div>

            {/* Status Filter */}
            <div className="flex gap-1">
              {(["all", "ok", "error"] as StatusFilter[]).map((f) => (
                <Button
                  key={f}
                  variant={statusFilter === f ? "default" : "outline"}
                  size="sm"
                  onClick={() => { setStatusFilter(f); setPage(0) }}
                >
                  {t(`logs.status.${f}`, f)}
                </Button>
              ))}
            </div>

            {/* Spacer */}
            <div className="flex-1" />

            {/* Actions */}
            <div data-tour="logs-actions">
            <Button variant="outline" size="sm" onClick={() => refetch()}>
              <RefreshCw className="h-4 w-4 mr-1" />
              {t("logs.refresh", "Refresh")}
            </Button>
            <Button variant="outline" size="sm" onClick={exportCsv} disabled={logs.length === 0}>
              <Download className="h-4 w-4 mr-1" />
              {t("logs.export", "Export CSV")}
            </Button>
            <Button variant="outline" size="sm" onClick={() => setShowClearModal(true)}>
              <Trash2 className="h-4 w-4 mr-1" />
              {t("logs.clearOld", "Clear Old")}
            </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Logs Table */}
      <Card>
        <CardContent className="p-0" data-tour="logs-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-8"></TableHead>
                <TableHead className="w-40">{t("logs.time", "Time")}</TableHead>
                <TableHead className="w-24">{t("logs.provider", "Provider")}</TableHead>
                <TableHead>{t("logs.model", "Model")}</TableHead>
                <TableHead>{t("logs.endpoint", "Endpoint")}</TableHead>
                <TableHead className="w-20 text-right">{t("logs.status", "Status")}</TableHead>
                <TableHead className="w-24 text-right">{t("logs.tokens", "Tokens")}</TableHead>
                <TableHead className="w-20 text-right">{t("logs.duration", "Duration")}</TableHead>
                <TableHead>{t("logs.error", "Error")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={9} className="text-center text-muted-foreground py-12">
                    {t("logs.loading", "Loading...")}
                  </TableCell>
                </TableRow>
              ) : logs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={9} className="text-center text-muted-foreground py-12">
                    {t("logs.empty", "No logs found")}
                  </TableCell>
                </TableRow>
              ) : (
                logs.map((log) => (
                  <LogRow
                    key={log.id}
                    log={log}
                    isExpanded={selectedLogId === log.id}
                    onToggle={() => setSelectedLogId(selectedLogId === log.id ? null : log.id)}
                  />
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex justify-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={page === 0}
            onClick={() => setPage(0)}
          >
            {t("logs.first", "First")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page === 0}
            onClick={() => setPage(page - 1)}
          >
            {t("logs.prev", "Previous")}
          </Button>
          <span className="flex items-center px-4 text-sm text-muted-foreground">
            {t("logs.page", "Page")} {page + 1} / {totalPages} ({total} total)
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages - 1}
            onClick={() => setPage(page + 1)}
          >
            {t("logs.next", "Next")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages - 1}
            onClick={() => setPage(totalPages - 1)}
          >
            {t("logs.last", "Last")}
          </Button>
        </div>
      )}

      {/* Clear Old Modal */}
      {showClearModal && <ClearOldModal onClose={() => setShowClearModal(false)} />}
    </div>
    </PageTour>
  )
}

function LogRow({ log, isExpanded, onToggle }: { log: LogRow; isExpanded: boolean; onToggle: () => void }) {
  const statusClass = log.status_code >= 500
    ? "bg-red-100 text-red-800"
    : log.status_code >= 400
    ? "bg-amber-100 text-amber-800"
    : "bg-emerald-100 text-emerald-800"

  return (
    <>
      <TableRow className="cursor-pointer hover:bg-muted/50">
        <TableCell>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onToggle}>
            {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </Button>
        </TableCell>
        <TableCell className="text-muted-foreground text-xs whitespace-nowrap">
          {new Date(log.ts).toLocaleString()}
        </TableCell>
        <TableCell>
          <Badge variant="outline">{log.provider_id}</Badge>
        </TableCell>
        <TableCell>
          <code className="text-xs">{log.client_model}</code>
        </TableCell>
        <TableCell>
          <code className="text-xs text-muted-foreground">{log.endpoint}</code>
        </TableCell>
        <TableCell className="text-right">
          <Badge className={cn(statusClass)}>
            {log.status_code}
            {log.stream && " · S"}
          </Badge>
        </TableCell>
        <TableCell className="text-right text-muted-foreground text-xs">
          {log.total_tokens !== null ? log.total_tokens.toLocaleString() : "—"}
        </TableCell>
        <TableCell className="text-right text-muted-foreground text-xs">
          {log.duration_ms}ms
        </TableCell>
        <TableCell>
          {log.error_code && (
            <code className="text-xs text-destructive" title={log.error_snippet || ""}>
              {log.error_code}
            </code>
          )}
        </TableCell>
      </TableRow>

      {/* Expanded Detail Row */}
      {isExpanded && (
        <TableRow>
          <TableCell colSpan={9} className="bg-muted/30 p-4">
            <LogDetailPanel logId={log.id} />
          </TableCell>
        </TableRow>
      )}
    </>
  )
}

function LogDetailPanel({ logId }: { logId: number }) {
  const { t } = useTranslation()
  const [view, setView] = useState<"structured" | "raw">("structured")

  const { data, isLoading } = useQuery({
    queryKey: ["log-detail", logId],
    queryFn: () => api.logs.detail(logId),
  })

  if (isLoading) {
    return <div className="text-center py-4 text-muted-foreground">{t("logs.loading", "Loading...")}</div>
  }

  if (!data?.log) {
    return <div className="text-center py-4 text-muted-foreground">Log not found</div>
  }

  const { log } = data

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <span className="font-medium">Log #{log.id}</span>
        <div className="flex gap-1 ml-auto">
          <Button
            variant={view === "structured" ? "default" : "outline"}
            size="sm"
            onClick={() => setView("structured")}
          >
            {t("logs.structured", "Structured")}
          </Button>
          <Button
            variant={view === "raw" ? "default" : "outline"}
            size="sm"
            onClick={() => setView("raw")}
          >
            {t("logs.raw", "Raw")}
          </Button>
        </div>
      </div>

      {view === "structured" ? (
        <StructuredDetail detail={log} />
      ) : (
        <div className="space-y-4">
          {log.request_body && (
            <div>
              <div className="font-medium text-sm mb-1">Request Body:</div>
              <pre className="bg-background border rounded p-2 text-xs overflow-auto max-h-64">
                {log.request_body}
              </pre>
            </div>
          )}
          {log.response_body && (
            <div>
              <div className="font-medium text-sm mb-1">Response Body:</div>
              <pre className="bg-background border rounded p-2 text-xs overflow-auto max-h-64">
                {log.response_body}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function ClearOldModal({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation()
  const [days, setDays] = useState(7)
  const [isDeleting, setIsDeleting] = useState(false)
  const queryClient = useQueryClient()

  async function handleClear() {
    setIsDeleting(true)
    try {
      const before = Date.now() - days * 24 * 60 * 60 * 1000
      await api.logs.deleteBefore(before)
      queryClient.invalidateQueries({ queryKey: ["logs"] })
      onClose()
    } catch (e) {
      console.error("Failed to clear logs:", e)
    } finally {
      setIsDeleting(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-96">
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            {t("logs.clearOldTitle", "Clear Old Logs")}
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            {t("logs.clearOldDesc", "Delete logs older than the specified number of days.")}
          </p>
          <div>
            <label className="text-sm font-medium mb-1 block">Days</label>
            <Input
              type="number"
              min={1}
              max={365}
              value={days}
              onChange={(e) => setDays(parseInt(e.target.value) || 7)}
            />
          </div>
          <div className="flex gap-2 justify-end">
            <Button variant="outline" onClick={onClose}>{t("logs.cancel", "Cancel")}</Button>
            <Button
              variant="destructive"
              onClick={handleClear}
              disabled={isDeleting}
            >
              {isDeleting ? t("logs.deleting", "Deleting...") : t("logs.delete", "Delete")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}