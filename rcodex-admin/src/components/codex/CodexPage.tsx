import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { AlertTriangle } from "lucide-react"
import { Switch } from "@/components/ui/switch"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Loader2, CheckCircle, XCircle, RefreshCw, Trash2, RotateCcw, Zap, Download, CloudUpload, CloudDownload } from "lucide-react"
import type { CodexState, CodexTarget, ProbeResult, BackupPair, CodexHistoryEntry } from "@/types/codex"
import { ImportModal, ExportModal } from "./ImportModal"
import { SetupSnippets } from "./SetupSnippets"
import { LanguageSwitcher } from "@/components/LanguageSwitcher"
import { PageTour, type TourStep } from "@/components/PageTour"
import { Info, Space } from "lucide-react"

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

// Download blob helper function
function downloadBlob(content: string, filename: string, mime = "text/plain"): void {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function CodexStateCard({ state, onRefresh }: { state: CodexState | undefined; onRefresh: () => void }) {
  const { t } = useTranslation()
  const [editingDir, setEditingDir] = useState(false)
  const [dirInput, setDirInput] = useState("")
  const ownerVariant = state?.authJsonOwner === "mimo2codex" ? "success" : state?.authJsonOwner === "external" ? "warning" : "secondary"
  const ownerLabel = state?.authJsonOwner === "mimo2codex" ? t("auth.mimo2codex") : state?.authJsonOwner === "external" ? t("auth.external") : t("auth.missing")

  const parseModelProvider = (text: string | null) => {
    if (!text) return { model: null, provider: null }
    const modelMatch = /^\s*model\s*=\s*"([^"\n]+)"/m.exec(text)
    const providerMatch = /^\s*model_provider\s*=\s*"([^"\n]+)"/m.exec(text)
    return { model: modelMatch?.[1] ?? null, provider: providerMatch?.[1] ?? null }
  }
  const parsed = state?.configTomlText ? parseModelProvider(state.configTomlText) : { model: null, provider: null }

  const handleExport = () => {
    if (state?.configTomlText) {
      downloadBlob(state.configTomlText, "config.toml", "text/plain")
    }
    if (state?.authJsonExists) {
      api.codex.state().then(s => {
        if (s.ok && s.data) {
          downloadBlob(JSON.stringify({ owner: s.data.authJsonOwner }, null, 2), "auth.json", "application/json")
        }
      })
    }
  }

  const handleSetDir = async () => {
    if (!dirInput.trim()) return
    try {
      await api.codex.setCodexDir(dirInput.trim())
      setEditingDir(false)
      onRefresh()
    } catch (e) {
      console.error("Failed to set codex dir:", e)
    }
  }

  const handleClearDir = async () => {
    try {
      await api.codex.clearCodexDir()
      setEditingDir(false)
      onRefresh()
    } catch (e) {
      console.error("Failed to clear codex dir:", e)
    }
  }

  return (
    <Card data-tour="codex-config">
      <CardHeader className="flex flex-row items-center justify-between">
        <div>
          <CardTitle>{t("codexState.title")}</CardTitle>
          <CardDescription>{t("codexState.description")}</CardDescription>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={handleExport} title={t("codexState.export")}>
            <Download className="h-4 w-4 mr-1" /> {t("codexState.export")}
          </Button>
          <Button variant="outline" size="icon" onClick={onRefresh}>
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.codexDir")}</p>
            {editingDir ? (
              <div className="flex gap-2 mt-1">
                <input
                  type="text"
                  value={dirInput}
                  onChange={(e) => setDirInput(e.target.value)}
                  placeholder={state?.codexDir}
                  className="flex h-8 w-full rounded-md border border-input bg-background px-3 py-1 text-sm font-mono"
                  autoFocus
                />
                <Button size="sm" onClick={handleSetDir}>Save</Button>
                <Button size="sm" variant="outline" onClick={() => setEditingDir(false)}>Cancel</Button>
                <Button size="sm" variant="ghost" onClick={handleClearDir}>Reset</Button>
              </div>
            ) : (
              <div className="flex items-center gap-2 mt-1">
                <p className="font-mono text-sm">{state?.codexDir ?? "-"}</p>
                <Button size="sm" variant="ghost" onClick={() => { setEditingDir(true); setDirInput(state?.codexDir || "") }}>Edit</Button>
              </div>
            )}
            <Badge variant="secondary" className="mt-1">{state?.codexDirSource ?? "default"}</Badge>
          </div>
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.authOwner")}</p>
            <Badge variant={ownerVariant as any} className="mt-1">{ownerLabel}</Badge>
            <p className="text-xs text-muted-foreground mt-1 font-mono truncate">{state?.authPath ?? "-"}</p>
          </div>
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.provider")}</p>
            <Badge variant="outline">{parsed.provider ?? "-"}</Badge>
          </div>
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.model")}</p>
            <Badge variant="secondary">{parsed.model ?? "-"}</Badge>
          </div>
        </div>
        <div className="flex gap-4">
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground">{t("codexState.configToml")}:</p>
            {state?.configTomlExists ? (
              <Badge variant="success">exists</Badge>
            ) : (
              <Badge variant="secondary">not found</Badge>
            )}
          </div>
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground">{t("codexState.backups")}:</p>
            <Badge variant="outline">{state?.backupPairs?.length ?? 0}</Badge>
          </div>
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground">Override:</p>
            <Badge variant="destructive">inactive</Badge>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

// Group targets by provider
function groupTargetsByProvider(targets: CodexTarget[]): Record<string, CodexTarget[]> {
  return targets.reduce((acc, target) => {
    if (!acc[target.providerId]) {
      acc[target.providerId] = []
    }
    acc[target.providerId].push(target)
    return acc
  }, {} as Record<string, CodexTarget[]>)
}

function ProviderSelector({ targets, onApply }: { targets: CodexTarget[]; onApply: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [selectedProvider, setSelectedProvider] = useState("")
  const [selectedModel, setSelectedModel] = useState("")
  const [probeResult, setProbeResult] = useState<ProbeResult | null>(null)
  const [probing, setProbing] = useState(false)
  const [testingAll, setTestingAll] = useState(false)
  const [probeResults, setProbeResults] = useState<Record<string, ProbeResult>>({})

  // Group targets by provider
  const groupedTargets = groupTargetsByProvider(targets)
  const providerIds = Object.keys(groupedTargets)

  const currentTarget = targets.find(t => t.providerId === selectedProvider && t.modelId === selectedModel)

  const handleProbe = async (providerId: string, modelId: string) => {
    setProbing(true)
    const key = `${providerId}::${modelId}`
    setProbeResults(prev => ({ ...prev, [key]: { ok: true, latencyMs: 0, error: undefined } }))
    try {
      const result = await api.codex.probe(providerId, modelId)
      if (result.ok && result.data) {
        setProbeResults(prev => ({ ...prev, [key]: result.data! }))
        if (providerId === selectedProvider && modelId === selectedModel) {
          setProbeResult(result.data)
        }
      }
    } catch (e) {
      const errResult = { ok: false, latencyMs: 0, error: { code: "error", message: String(e) } }
      setProbeResults(prev => ({ ...prev, [key]: errResult }))
      if (providerId === selectedProvider && modelId === selectedModel) {
        setProbeResult(errResult)
      }
    }
    setProbing(false)
  }

  const handleTestAll = async () => {
    setTestingAll(true)
    setProbeResults({})
    await Promise.all(targets.map(async (target) => {
      try {
        const result = await api.codex.probe(target.providerId, target.modelId)
        if (result.ok && result.data) {
          setProbeResults(prev => ({ ...prev, [`${target.providerId}::${target.modelId}`]: result.data! }))
        }
      } catch (e) {
        const key = `${target.providerId}::${target.modelId}`
        setProbeResults(prev => ({ ...prev, [key]: { ok: false, latencyMs: 0, error: { code: "error", message: String(e) } } }))
      }
    }))
    setTestingAll(false)
  }

  const handleSelect = (providerId: string, modelId: string) => {
    setSelectedProvider(providerId)
    setSelectedModel(modelId)
  }

  const handleApply = async () => {
    if (!selectedProvider || !selectedModel) return
    try {
      await api.codex.apply(selectedProvider, selectedModel)
      queryClient.invalidateQueries({ queryKey: ["codex-state"] })
      onApply()
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <Card data-tour="codex-providers">
      <CardHeader>
        <CardTitle>{t("provider.title")}</CardTitle>
        <CardDescription>{t("provider.description")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Test All Button */}
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={handleTestAll} disabled={testingAll || targets.length === 0}>
            {testingAll ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Zap className="mr-2 h-4 w-4" />}
            {testingAll ? t("provider.testingAll") : t("provider.testAll")}
          </Button>
          {Object.keys(probeResults).length > 0 && (
            <span className="text-sm text-muted-foreground">
              {`${Object.values(probeResults).filter(r => r.ok).length}/${Object.keys(probeResults).length} OK`}
            </span>
          )}
        </div>

        {/* External Warning */}
        {targets.some(t => t.source !== "builtin") && (
          <Alert variant="default" className="border-yellow-300 bg-yellow-50">
            <AlertTriangle className="h-4 w-4 text-yellow-600" />
            <AlertDescription className="text-yellow-800 text-sm">
              {t("provider.externalWarning")}
            </AlertDescription>
          </Alert>
        )}

        {/* Provider Groups Table */}
        {providerIds.map(providerId => {
          const models = groupedTargets[providerId]
          return (
            <div key={providerId} className="border rounded-lg overflow-hidden">
              <div className="bg-muted px-4 py-2 font-medium flex items-center justify-between">
                <span>{models[0]?.providerName ?? providerId}</span>
                <Badge variant="outline">{providerId}</Badge>
              </div>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Model</TableHead>
                    <TableHead>Source</TableHead>
                    <TableHead>Context</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {models.map(target => {
                    const key = `${target.providerId}::${target.modelId}`
                    const result = probeResults[key]
                    const isSelected = selectedProvider === target.providerId && selectedModel === target.modelId
                    return (
                      <TableRow key={key} className={isSelected ? "bg-primary/10" : ""}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <code className="text-sm">{target.displayName ?? target.modelId}</code>
                            {target.isCurrentOverride && (
                              <Badge variant="success" className="text-xs">Active</Badge>
                            )}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant={target.source === "builtin" ? "outline" : "secondary"}>
                            {target.source}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          {target.contextWindow ? (
                            <span className="text-sm text-muted-foreground">
                              {target.contextWindow.toLocaleString()}
                            </span>
                          ) : (
                            <span className="text-muted-foreground">—</span>
                          )}
                        </TableCell>
                        <TableCell>
                          {!target.hasKey ? (
                            <Badge variant="destructive" className="text-xs">No Key</Badge>
                          ) : result ? (
                            result.ok ? (
                              <Badge variant="success">{result.latencyMs}ms</Badge>
                            ) : (
                              <Badge variant="destructive">Failed</Badge>
                            )
                          ) : (
                            <Badge variant="secondary">Not tested</Badge>
                          )}
                        </TableCell>
                        <TableCell>
                          <div className="flex gap-2">
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => handleProbe(target.providerId, target.modelId)}
                              disabled={probing || !target.hasKey}
                              title={!target.hasKey ? "No API key configured" : "Test connection"}
                            >
                              {probing ? <Loader2 className="h-3 w-3 animate-spin" /> : <Zap className="h-3 w-3" />}
                            </Button>
                            <Button
                              size="sm"
                              variant={isSelected ? "default" : "outline"}
                              onClick={() => handleSelect(target.providerId, target.modelId)}
                              disabled={!target.hasKey}
                            >
                              Select
                            </Button>
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => {
                                // TODO: Implement override
                                console.log("Override", target.providerId, target.modelId)
                              }}
                              title="Set as runtime override"
                            >
                              Override
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    )
                  })}
                </TableBody>
              </Table>
            </div>
          )
        })}

        {/* Apply Button */}
        <div className="flex gap-2">
          <Button onClick={handleApply} disabled={!selectedProvider || !selectedModel}>
            {t("provider.applyCodex")}
          </Button>
        </div>

        {/* Selected Target Info */}
        {currentTarget && (
          <div className="p-3 bg-muted rounded-lg">
            <p className="text-sm font-medium">{t("provider.targetUrl")}:</p>
            <p className="text-xs font-mono text-muted-foreground">{currentTarget.baseUrl}</p>
          </div>
        )}

        {/* Probe Result Alert */}
        {probeResult && (
          <Alert variant={probeResult.ok ? "default" : "destructive"}>
            {probeResult.ok ? <CheckCircle className="h-4 w-4" /> : <XCircle className="h-4 w-4" />}
            <AlertTitle>{probeResult.ok ? t("probe.success") : t("probe.failed")}</AlertTitle>
            <AlertDescription>
              <p>{t("probe.latency")}: {probeResult.latencyMs}ms</p>
              {probeResult.error && <p className="text-sm mt-1">{probeResult.error.message}</p>}
            </AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  )
}

function BackupList({ pairs }: { pairs: BackupPair[] }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const restoreMutation = useMutation({
    mutationFn: async (ts: number) => api.codex.restore(ts),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["codex-state"] }),
  })
  const deleteMutation = useMutation({
    mutationFn: async (ts: number) => api.codex.deleteBackup(ts),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["codex-state"] }),
  })

  return (
    <Card data-tour="backups">
      <CardHeader>
        <CardTitle>{t("backup.title")}</CardTitle>
        <CardDescription>{t("backup.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        {pairs.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("backup.timestamp")}</TableHead>
                <TableHead>{t("backup.time")}</TableHead>
                <TableHead>{t("backup.type")}</TableHead>
                <TableHead>{t("backup.providerModel")}</TableHead>
                <TableHead>{t("backup.actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {pairs.map((pair) => (
                <TableRow key={pair.ts}>
                  <TableCell className="font-mono text-xs">{pair.ts}</TableCell>
                  <TableCell>{new Date(pair.ts).toLocaleString()}</TableCell>
                  <TableCell>
                    {pair.preserved ? <Badge variant="success">{t("backup.preserved")}</Badge> : <Badge variant="outline">{t("backup.snapshot")}</Badge>}
                  </TableCell>
                  <TableCell>
                    <code className="text-xs">{pair.provider ?? "?"} / {pair.model ?? "?"}</code>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-2">
                      <Button size="sm" variant="outline" onClick={() => restoreMutation.mutate(pair.ts)}>
                        <RotateCcw className="mr-1 h-4 w-4" /> {t("backup.restore")}
                      </Button>
                      <Button size="sm" variant="destructive" onClick={() => {
                        if (confirm(pair.preserved ? t("backup.confirmDeletePreserved") : t("backup.confirmDelete"))) {
                          deleteMutation.mutate(pair.ts)
                        }
                      }}>
                        <Trash2 className="mr-1 h-4 w-4" /> {t("backup.delete")}
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <div className="text-center py-8 text-muted-foreground">
            <p>{t("backup.noBackups")}</p>
            <p className="text-sm">{t("backup.noBackupsHint")}</p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function OverridePanel() {
  const queryClient = useQueryClient()
  const [providerId, setProviderId] = useState("")
  const [modelId, setModelId] = useState("")

  const { data: override } = useQuery({
    queryKey: ["active-override"],
    queryFn: () => api.codex.activeOverride(),
  })

  const setMutation = useMutation({
    mutationFn: async () => api.codex.setOverride(providerId, modelId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["active-override"] }),
  })

  const clearMutation = useMutation({
    mutationFn: async () => api.codex.clearOverride(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["active-override"] }),
  })

  return (
    <Card data-tour="codex-override">
      <CardHeader>
        <CardTitle>Runtime Override</CardTitle>
        <CardDescription>Override provider/model at runtime (temporary)</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
          <div>
            {override?.data ? (
              <div className="flex items-center gap-2">
                <Badge variant="default">{override.data.providerId}</Badge>
                <span>/</span>
                <Badge variant="outline">{override.data.modelId}</Badge>
              </div>
            ) : (
              <p className="text-muted-foreground">No override active (using config)</p>
            )}
          </div>
          <Button size="sm" variant="outline" onClick={() => clearMutation.mutate()} disabled={!override?.data || clearMutation.isPending}>
            Clear Override
          </Button>
        </div>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="text-sm font-medium">Provider</label>
            <input placeholder="openai" value={providerId} onChange={(e) => setProviderId(e.target.value)}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm mt-1" />
          </div>
          <div>
            <label className="text-sm font-medium">Model</label>
            <input placeholder="gpt-4" value={modelId} onChange={(e) => setModelId(e.target.value)}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm mt-1" />
          </div>
        </div>
        <Button onClick={() => setMutation.mutate()} disabled={!providerId || !modelId || setMutation.isPending}>
          Set Override
        </Button>
      </CardContent>
    </Card>
  )
}

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

function ThinkingPanel() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [thinkingDisabled, setThinkingDisabled] = useState(false)
  const [forceHighEffort, setForceHighEffort] = useState(false)

  const { data: thinking } = useQuery({
    queryKey: ["thinking-state"],
    queryFn: () => api.codex.thinking(),
  })

  const updateThinking = useMutation({
    mutationFn: async (params: { disabled: boolean; forceHighEffort: boolean }) =>
      api.codex.setThinking(params.disabled, params.forceHighEffort),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["thinking-state"] }),
  })

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
                updateThinking.mutate({ disabled, forceHighEffort })
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
      // Ctrl+R or Cmd+R - Refresh
      if ((e.ctrlKey || e.metaKey) && e.key === "r") {
        e.preventDefault()
        queryClient.invalidateQueries({ queryKey: ["codex-state"] })
        showNotification("success", "Refreshed!")
      }
      // Ctrl+S or Cmd+S - Open Export
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault()
        setExportOpen(true)
      }
      // Escape - Close modals
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
    return (
      <div className="p-8">
        <Alert variant="destructive">
          <XCircle className="h-4 w-4" />
          <AlertTitle>Error</AlertTitle>
          <AlertDescription>
            {state?.error ?? "Failed to load Codex state"}
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
            <CodexStateCard state={state.data} onRefresh={handleRefresh} />
            {targets?.ok && targets.data && (
              <ProviderSelector targets={targets.data.targets} onApply={() => showNotification("success", "Codex applied successfully!")} />
            )}
          </TabsContent>

          <TabsContent value="setup" className="animate-fadeIn">
            <SetupSnippets />
          </TabsContent>

          <TabsContent value="thinking" className="animate-fadeIn">
            <ThinkingPanel />
          </TabsContent>

          <TabsContent value="backups" className="animate-fadeIn">
            <BackupList pairs={state.data?.backupPairs ?? []} />
          </TabsContent>

          <TabsContent value="history" className="animate-fadeIn">
            <HistoryPanel />
          </TabsContent>

          <TabsContent value="override">
            <OverridePanel />
          </TabsContent>
        </Tabs>
      </div>
    </PageTour>
  )
}
