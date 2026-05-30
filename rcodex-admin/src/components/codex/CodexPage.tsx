/**
 * CodexPage - Codex 配置管理页面
 *
 * 重构后的精简版本，使用拆分出的子组件：
 * - CurrentStateCard: 当前状态卡片
 * - ProviderBlock: Provider/Model 选择和测试
 * - RuntimeOverrideCard: 运行时覆盖
 * - BackupCard: 备份历史
 * - HistoryPanel: 配置历史
 * - ThinkingPanel: 思考模式
 */

import { useQuery, useQueryClient } from "@tanstack/react-query"
import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Switch } from "@/components/ui/switch"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Loader2, CheckCircle, XCircle, Trash2, Download, CloudUpload, CloudDownload } from "lucide-react"
import type { CodexHistoryEntry } from "@/types/codex"
import { ImportModal, ExportModal } from "./ImportModal"
import { SetupSnippets } from "./SetupSnippets"
import { LanguageSwitcher } from "@/components/LanguageSwitcher"
import { PageTour, type TourStep } from "@/components/PageTour"
import { Info, Space } from "lucide-react"
import { CurrentStateCard } from "./CurrentStateCard"
import { ProviderBlock } from "./ProviderBlock"
import { RuntimeOverrideCard } from "./RuntimeOverrideCard"
import { BackupCard } from "./BackupCard"

// Codex PageTour steps
const CODEX_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='codex-config']",
    title: "Configuration Status",
    description: "View your current Codex configuration including auth.json and config.toml status.",
    placement: "bottom",
  },
  {
    target: "[data-tour='codex-providers']",
    title: "Provider Selection",
    description: "Select a provider and model for Claude Code. Test connectivity before applying.",
    placement: "top",
  },
  {
    target: "[data-tour='codex-override']",
    title: "Runtime Override",
    description: "Temporarily override the provider/model without changing config files.",
    placement: "top",
  },
  {
    target: "[data-tour='codex-history']",
    title: "Configuration History",
    description: "View the audit trail of all configuration changes with restore capability.",
    placement: "top",
  },
]

// ============ History Panel ============

function HistoryPanel() {
  const { data: history, isLoading } = useQuery({
    queryKey: ["codex-history"],
    queryFn: () => api.codex.history(),
  })

  if (isLoading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin" />
        </CardContent>
      </Card>
    )
  }

  return (
    <Card data-tour="codex-history">
      <CardHeader>
        <CardTitle>Codex History</CardTitle>
        <CardDescription>Audit trail of configuration changes</CardDescription>
      </CardHeader>
      <CardContent>
        {history?.ok && history.data && history.data.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Time</TableHead>
                <TableHead>Kind</TableHead>
                <TableHead>Provider</TableHead>
                <TableHead>Model</TableHead>
                <TableHead>Note</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {history.data.map((entry: CodexHistoryEntry) => (
                <TableRow key={entry.id}>
                  <TableCell className="font-mono text-sm">{new Date(entry.createdAt).toLocaleString()}</TableCell>
                  <TableCell>
                    <Badge variant={entry.kind === "Apply" ? "default" : entry.kind === "Restore" ? "secondary" : "outline"}>
                      {entry.kind}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono text-sm">{(entry as any).provider_id ?? "-"}</TableCell>
                  <TableCell className="font-mono text-sm">{(entry as any).model_id ?? "-"}</TableCell>
                  <TableCell className="text-muted-foreground">{entry.note ?? ""}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button size="sm" variant="ghost"><Download className="h-4 w-4" /></Button>
                      {entry.kind !== "Initial" && (
                        <Button size="sm" variant="ghost" className="text-destructive"><Trash2 className="h-4 w-4" /></Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <div className="text-center py-8 text-muted-foreground">
            <p>No history entries yet</p>
            <p className="text-sm">Apply or restore configurations to see history</p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ============ Thinking Panel ============

function ThinkingPanel() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [thinkingDisabled, setThinkingDisabled] = useState(false)
  const [forceHighEffort, setForceHighEffort] = useState(false)

  const { data: thinking } = useQuery({
    queryKey: ["thinking-state"],
    queryFn: () => api.codex.thinking(),
  })

  const updateThinking = useQueryClient.prototype ? {} : {
    mutate: async (params: { disabled: boolean; forceHighEffort: boolean }) => {
      try {
        await api.codex.setThinking(params.disabled, params.forceHighEffort)
        queryClient.invalidateQueries({ queryKey: ["thinking-state"] })
      } catch (e) {
        console.error("Failed to update thinking:", e)
      }
    }
  }

  const cliOverride = thinking?.data?.cli_override ?? false

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("thinking.title")}</CardTitle>
        <CardDescription>{t("thinking.description")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {cliOverride && (
          <Alert variant="default" className="border-yellow-500 bg-yellow-50">
            <AlertTitle className="text-yellow-800">CLI Override Active</AlertTitle>
            <AlertDescription className="text-yellow-700 text-sm">
              Thinking mode is controlled by CLI environment variables. UI controls are disabled.
            </AlertDescription>
          </Alert>
        )}

        <div className="flex items-center justify-between p-4 bg-muted rounded-lg" data-tour="thinking">
          <div className="space-y-0.5">
            <div className="font-medium">{t("thinking.enableThinking")}</div>
            <div className="text-sm text-muted-foreground">{t("thinking.enableThinkingDesc")}</div>
          </div>
          <div className="flex items-center gap-3">
            <span className={`text-sm font-medium ${!thinkingDisabled ? "text-green-600" : "text-muted-foreground"}`}>
              {thinkingDisabled ? t("thinking.statusOff") : t("thinking.statusOn")}
            </span>
            <Switch
              checked={!thinkingDisabled}
              disabled={cliOverride}
              onCheckedChange={(checked) => {
                const disabled = !checked
                setThinkingDisabled(disabled)
                updateThinking.mutate?.({ disabled, forceHighEffort })
              }}
            />
          </div>
        </div>

        <div className="flex items-center justify-between p-4 bg-muted rounded-lg">
          <div className="space-y-0.5">
            <div className="font-medium">{t("thinking.forceHighEffort")}</div>
            <div className="text-sm text-muted-foreground">{t("thinking.forceHighEffortDesc")}</div>
          </div>
          <Switch checked={forceHighEffort} onCheckedChange={setForceHighEffort} disabled={thinkingDisabled || cliOverride} />
        </div>

        {thinking?.ok && (
          <div className="text-xs text-muted-foreground space-y-1">
            <p>Current: disabled={thinking.data?.disabled ? "true" : "false"}</p>
            <p>forceHighEffort={thinking.data?.force_high_effort ? "true" : "false"}</p>
            <p>cli_override={thinking.data?.cli_override ? "true" : "false"}</p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ============ Main CodexPage ============

export function CodexPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [notification, setNotification] = useState<{ type: "success" | "error"; message: string } | null>(null)
  const [importOpen, setImportOpen] = useState(false)
  const [exportOpen, setExportOpen] = useState(false)
  const [activeTab, setActiveTab] = useState("config")

  const { data: state, isLoading: stateLoading, error: stateError } = useQuery({
    queryKey: ["codex-state"],
    queryFn: () => api.codex.state(),
  })

  const { data: targets, isLoading: targetsLoading } = useQuery({
    queryKey: ["codex-targets"],
    queryFn: () => api.codex.targets(),
  })

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "r") {
        e.preventDefault()
        queryClient.invalidateQueries({ queryKey: ["codex-state"] })
        showNotification("success", "Refreshed!")
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault()
        setExportOpen(true)
      }
      if (e.key === "Escape") {
        setImportOpen(false)
        setExportOpen(false)
      }
    }
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [queryClient])

  const showNotification = (type: "success" | "error", message: string) => {
    setNotification({ type, message })
    setTimeout(() => setNotification(null), 3000)
  }

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: ["codex-state"] })
    showNotification("success", "Refreshed!")
  }

  if (stateLoading || targetsLoading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (stateError || !state?.ok) {
    const errorMsg = state?.error
      ? typeof state.error === "string"
        ? state.error
        : (state.error as { code?: string; message?: string; type?: string }).message || JSON.stringify(state.error)
      : "Failed to load Codex state"
    return (
      <div className="p-8">
        <Alert variant="destructive">
          <XCircle className="h-4 w-4" />
          <AlertTitle>Error</AlertTitle>
          <AlertDescription>
            {errorMsg}
          </AlertDescription>
        </Alert>
      </div>
    )
  }

  return (
    <PageTour pageKey="codex" steps={CODEX_TOUR_STEPS}>
      <div className="container mx-auto py-8 space-y-6">
        {notification && (
          <Alert variant={notification.type === "success" ? "default" : "destructive"} className="fixed top-4 right-4 z-50">
            {notification.type === "success" ? <CheckCircle className="h-4 w-4" /> : <XCircle className="h-4 w-4" />}
            <AlertTitle>{notification.type === "success" ? "Success" : "Error"}</AlertTitle>
            <AlertDescription>{notification.message}</AlertDescription>
          </Alert>
        )}

        {/* Info Alert */}
        <Alert variant="default" className="border-blue-200 bg-blue-50">
          <Info className="h-4 w-4 text-blue-600" />
          <AlertDescription className="text-blue-800">
            <Space className="inline-flex flex-col gap-1">
              <span>{t("app.modesInfo.applyFile")}</span>
              <span>{t("app.modesInfo.runtimeOverride")}</span>
            </Space>
          </AlertDescription>
        </Alert>

        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">{t("app.title")}</h1>
            <p className="text-muted-foreground">{t("app.subtitle")}</p>
          </div>
          <div className="flex gap-2">
            <LanguageSwitcher />
            <Button variant="outline" size="sm" onClick={() => setExportOpen(true)}>
              <CloudDownload className="h-4 w-4 mr-1" /> {t("export.title")}
            </Button>
            <Button variant="outline" size="sm" onClick={() => setImportOpen(true)}>
              <CloudUpload className="h-4 w-4 mr-1" /> {t("import.title")}
            </Button>
          </div>
        </div>

        <ImportModal
          open={importOpen}
          onClose={() => setImportOpen(false)}
          onImported={() => {
            queryClient.invalidateQueries({ queryKey: ["codex-state"] })
            showNotification("success", "Configuration imported successfully!")
          }}
        />

        <ExportModal
          open={exportOpen}
          onClose={() => setExportOpen(false)}
        />

        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList>
            <TabsTrigger value="config">{t("nav.config")}</TabsTrigger>
            <TabsTrigger value="setup">{t("nav.setup")}</TabsTrigger>
            <TabsTrigger value="thinking">{t("nav.thinking")}</TabsTrigger>
            <TabsTrigger value="backups">{t("nav.backups")}</TabsTrigger>
            <TabsTrigger value="history">{t("nav.history")}</TabsTrigger>
            <TabsTrigger value="override">{t("nav.override")}</TabsTrigger>
          </TabsList>

          <TabsContent value="config" className="space-y-4 animate-fadeIn">
            <CurrentStateCard state={state.data} onRefresh={handleRefresh} />
            {targets?.ok && targets.data && (
              <ProviderBlock targets={targets.data.targets} onApplied={() => showNotification("success", "Codex applied successfully!")} />
            )}
          </TabsContent>

          <TabsContent value="setup" className="animate-fadeIn">
            <SetupSnippets />
          </TabsContent>

          <TabsContent value="thinking" className="animate-fadeIn">
            <ThinkingPanel />
          </TabsContent>

          <TabsContent value="backups" className="animate-fadeIn">
            <BackupCard pairs={state.data?.backupPairs ?? []} />
          </TabsContent>

          <TabsContent value="history" className="animate-fadeIn">
            <HistoryPanel />
          </TabsContent>

          <TabsContent value="override">
            <RuntimeOverrideCard />
          </TabsContent>
        </Tabs>
      </div>
    </PageTour>
  )
}
