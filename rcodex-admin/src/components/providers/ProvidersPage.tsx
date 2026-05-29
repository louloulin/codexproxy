import { useTranslation } from "react-i18next"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useState } from "react"
import { Plus, Pencil, Trash2, Zap, X, Check, AlertCircle, Code, ChevronDown, ChevronRight, Loader2 } from "lucide-react"
import { api } from "@/lib/api"
import { RawJsonEditor } from "./RawJsonEditor"
import { PageTour, type TourStep } from "@/components/PageTour"
import type { ProbeResult } from "@/types/codex"

type Model = {
  upstream_id: string
  display_name: string | null
  context_window: number | null
  supports_images: boolean
  supports_reasoning: boolean
}

// Providers PageTour steps
const PROVIDERS_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='providers-actions']",
    title: "Provider Actions",
    description: "Add new providers manually or use Raw JSON to edit configurations directly. Built-in providers cannot be deleted.",
    placement: "bottom",
  },
  {
    target: "[data-tour='providers-table']",
    title: "Providers Table",
    description: "View and manage configured providers. Click the edit icon to modify settings or use Raw JSON for bulk operations.",
    placement: "top",
  },
  {
    target: "[data-tour='providers-presets']",
    title: "Quick Presets",
    description: "Use presets to quickly configure common providers like OpenAI, Anthropic, DeepSeek, or Google AI.",
    placement: "top",
  },
]

type Provider = {
  id: string
  display_name: string
  base_url?: string
  endpoint_base?: string
  auth_type?: "bearer" | "api-key"
  api_key_present?: boolean
  is_builtin?: boolean
  default_model?: string
}

// Icon mapping for provider shortcuts
const PRESET_ICONS: Record<string, string> = {
  openai: "🤖",
  anthropic: "🧠",
  deepseek: "🔮",
  google: "🌐",
  gemini: "✨",
  ollama: "🦙",
  "open-router": "🛣️",
  groq: "⚡",
  mistral: "🌬️",
  cohere: "🌊",
  fireworks: "🎆",
  novita: "🆕",
  default: "🔌",
}

export function ProvidersPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [editingProvider, setEditingProvider] = useState<string | null>(null)
  const [showAddModal, setShowAddModal] = useState(false)
  const [expandedProvider, setExpandedProvider] = useState<string | null>(null)
  const [pendingPreset, setPendingPreset] = useState<{
    id: string
    name: string
    shortcut?: string
    endpoint: string
    auth: "bearer" | "api-key"
    icon: string
  } | null>(null)
  const [showRawJson, setShowRawJson] = useState(false)
  const [rawJsonValue, setRawJsonValue] = useState("")

  // Use provider-configs API which returns array format
  const { data, isLoading } = useQuery({
    queryKey: ["provider-configs"],
    queryFn: () => api.providers.config(),
  })

  // Fetch presets from API
  const { data: presetsData } = useQuery({
    queryKey: ["provider-presets"],
    queryFn: () => api.providers.presets(),
  })

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      await api.providers.delete(id)
      return id
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["provider-configs"] })
    },
  })

  // Extract providers array from response
  const providers: Provider[] = data?.providers ?? []

  // Open raw JSON editor
  function openRawJson() {
    const json = JSON.stringify(
      providers.map((p) => ({
        id: p.id,
        display_name: p.display_name,
        endpoint_base: p.endpoint_base || p.base_url,
        auth_type: p.auth_type,
      })),
      null,
      2
    )
    setRawJsonValue(json)
    setShowRawJson(true)
  }

  // Save raw JSON
  async function handleSaveRawJson() {
    try {
      const parsed = JSON.parse(rawJsonValue)
      // Save each provider
      for (const p of parsed) {
        const existing = providers.find((ep) => ep.id === p.id)
        if (existing) {
          await api.providers.update(p.id, {
            display_name: p.display_name,
            endpoint_base: p.endpoint_base,
            auth_type: p.auth_type,
          })
        } else {
          await api.providers.create({
            id: p.id,
            display_name: p.display_name || p.id,
            endpoint_base: p.endpoint_base,
            auth_type: p.auth_type || "bearer",
          })
        }
      }
      queryClient.invalidateQueries({ queryKey: ["provider-configs"] })
      setShowRawJson(false)
    } catch (e) {
      console.error("Failed to save raw JSON:", e)
    }
  }

  return (
    <PageTour pageKey="providers" steps={PROVIDERS_TOUR_STEPS}>
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("providers.title", "Providers")}</h1>
        <p className="text-muted-foreground">
          {t("providers.subtitle", "Configure API providers and authentication")}
        </p>
      </div>

      {/* Add Provider Button */}
      <div className="flex justify-between items-center" data-tour="providers-actions">
        <p className="text-sm text-muted-foreground">
          {t("providers.hint", "Configure the API providers that rcodex can use as upstream backends.")}
        </p>
        <div className="flex gap-2">
          <Button variant="outline" onClick={openRawJson}>
            <Code className="h-4 w-4 mr-1" />
            {t("providers.rawJson", "Raw JSON")}
          </Button>
          <Button onClick={() => setShowAddModal(true)}>
            <Plus className="h-4 w-4 mr-1" />
            {t("providers.add", "Add Provider")}
          </Button>
        </div>
      </div>

      {/* Providers Table */}
      <Card>
        <CardContent className="p-0" data-tour="providers-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("providers.name", "Provider")}</TableHead>
                <TableHead>{t("providers.endpoint", "Endpoint")}</TableHead>
                <TableHead>{t("providers.auth", "Auth")}</TableHead>
                <TableHead className="text-right">{t("providers.actions", "Actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center text-muted-foreground py-12">
                    {t("providers.loading", "Loading...")}
                  </TableCell>
                </TableRow>
              ) : providers.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center text-muted-foreground py-12">
                    {t("providers.empty", "No providers configured. Add one to get started.")}
                  </TableCell>
                </TableRow>
              ) : (
                providers.map((provider) => (
                  <ProviderRow
                    key={provider.id}
                    provider={provider}
                    onEdit={() => setEditingProvider(provider.id)}
                    onDelete={() => deleteMutation.mutate(provider.id)}
                    isDeleting={deleteMutation.isPending}
                    isExpanded={expandedProvider === provider.id}
                    onToggleExpand={() => setExpandedProvider(expandedProvider === provider.id ? null : provider.id)}
                  />
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Presets Section */}
      <Card>
        <CardHeader>
          <CardTitle>{t("providers.presets", "Quick Presets")}</CardTitle>
          <CardDescription>
            {t("providers.presetsDesc", "Click to quick-fill provider settings")}
          </CardDescription>
        </CardHeader>
        <CardContent data-tour="providers-presets">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {(presetsData?.presets ?? []).map((preset: { id: string; name: string; shortcut?: string; defaultBaseUrl?: string; defaultModel?: string }) => {
              const icon = PRESET_ICONS[preset.shortcut || preset.id] || PRESET_ICONS.default
              return (
              <button
                key={preset.id}
                className="flex flex-col items-center gap-2 p-4 border rounded-lg hover:bg-muted transition-colors"
                onClick={() => {
                  setShowAddModal(true)
                  setPendingPreset({ id: preset.id, name: preset.name, shortcut: preset.shortcut, endpoint: preset.defaultBaseUrl || "", auth: "bearer", icon })
                }}
              >
                <span className="text-2xl">{icon}</span>
                <span className="text-sm font-medium">{preset.name}</span>
              </button>
              )
            })}
          </div>
        </CardContent>
      </Card>

      {/* Add/Edit Modal */}
      {(showAddModal || editingProvider) && (
        <ProviderFormModal
          providerId={editingProvider}
          preset={editingProvider ? undefined : pendingPreset}
          providers={providers}
          onClose={() => {
            setShowAddModal(false)
            setEditingProvider(null)
            setPendingPreset(null)
          }}
          onSave={() => {
            queryClient.invalidateQueries({ queryKey: ["provider-configs"] })
          }}
        />
      )}

      {/* Raw JSON Editor Modal */}
      <RawJsonEditor
        showRawJson={showRawJson}
        rawJsonValue={rawJsonValue}
        setRawJsonValue={setRawJsonValue}
        setShowRawJson={setShowRawJson}
        onSave={handleSaveRawJson}
      />
    </div>
    </PageTour>
  )
}

function ProviderRow({
  provider,
  onEdit,
  onDelete,
  isDeleting,
  isExpanded,
  onToggleExpand,
}: {
  provider: Provider
  onEdit: () => void
  onDelete: () => void
  isDeleting: boolean
  isExpanded: boolean
  onToggleExpand: () => void
}) {
  const { t } = useTranslation()
  const [isTesting, setIsTesting] = useState(false)
  const [probeResult, setProbeResult] = useState<ProbeResult | null>(null)
  const [expandedModels, setExpandedModels] = useState<Model[]>([])
  const [loadingModels, setLoadingModels] = useState(false)

  async function handleTestConnection() {
    if (!provider.default_model) return
    setIsTesting(true)
    setProbeResult(null)
    try {
      const result = await api.codex.probe(provider.id, provider.default_model)
      setProbeResult(result.data ?? null)
    } catch (e) {
      setProbeResult({ ok: false, latencyMs: 0, error: { code: "test_failed", message: String(e) } })
    } finally {
      setIsTesting(false)
    }
  }

  async function handleToggleExpand() {
    if (isExpanded) {
      onToggleExpand()
      return
    }
    setLoadingModels(true)
    try {
      // Use codex-targets as providers.models returns empty
      const result = await api.codex.targets()
      const targets = result.data?.targets ?? []
      // Filter models for this provider and transform to Model format
      const providerModels = targets
        .filter((t) => t.providerId === provider.id)
        .map((t) => ({
          upstream_id: t.modelId,
          display_name: t.displayName || null,
          context_window: t.contextWindow ?? null,
          supports_images: false, // Not available from targets
          supports_reasoning: false, // Not available from targets
        }))
      setExpandedModels(providerModels)
    } catch {
      setExpandedModels([])
    } finally {
      setLoadingModels(false)
      onToggleExpand()
    }
  }

  return (
    <>
      <TableRow className="cursor-pointer hover:bg-muted/50" onClick={handleToggleExpand}>
        <TableCell>
          <div className="flex items-center gap-2">
            {isExpanded ? (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            )}
            <span className="font-medium">{provider.display_name || provider.id}</span>
            {provider.is_builtin && (
              <Badge variant="outline" className="text-xs">
                {t("providers.builtin", "builtin")}
              </Badge>
            )}
            {provider.api_key_present && (
              <Check className="h-4 w-4 text-emerald-500" aria-label="API key configured" />
            )}
          </div>
        </TableCell>
        <TableCell>
          <code className="text-xs text-muted-foreground">
            {provider.base_url || provider.endpoint_base || "—"}
          </code>
        </TableCell>
        <TableCell>
          <Badge variant="secondary">{provider.auth_type === "api-key" ? "API Key" : "Bearer"}</Badge>
        </TableCell>
        <TableCell className="text-right" onClick={(e) => e.stopPropagation()}>
          <div className="flex justify-end gap-1">
            <Button
              size="sm"
              variant="ghost"
              onClick={handleTestConnection}
              disabled={isTesting || !provider.default_model}
              title="Test connection"
            >
              {isTesting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : probeResult ? (
                probeResult.ok ? (
                  <Check className="h-4 w-4 text-emerald-500" />
                ) : (
                  <AlertCircle className="h-4 w-4 text-destructive" />
                )
              ) : (
                <Zap className="h-4 w-4" />
              )}
            </Button>
            <Button size="sm" variant="ghost" onClick={onEdit}>
              <Pencil className="h-4 w-4" />
            </Button>
            {!provider.is_builtin && (
              <Button
                size="sm"
                variant="ghost"
                className="text-destructive"
                onClick={onDelete}
                disabled={isDeleting}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        </TableCell>
      </TableRow>
      {isExpanded && (
        <TableRow>
          <TableCell colSpan={4} className="bg-muted/30 p-4">
            {/* Test Result */}
            {probeResult && (
              <div className={`mb-4 p-3 rounded-lg ${probeResult.ok ? "bg-emerald-500/10" : "bg-destructive/10"}`}>
                <div className="flex items-center gap-2">
                  {probeResult.ok ? (
                    <Check className="h-4 w-4 text-emerald-500" />
                  ) : (
                    <AlertCircle className="h-4 w-4 text-destructive" />
                  )}
                  <span className="font-medium">
                    {probeResult.ok ? "Connection successful" : "Connection failed"}
                  </span>
                  <span className="text-sm text-muted-foreground">
                    ({probeResult.latencyMs}ms)
                  </span>
                </div>
                {probeResult.error && (
                  <p className="text-sm text-destructive mt-1">{probeResult.error.message}</p>
                )}
              </div>
            )}
            {/* Models */}
            <div>
              <h4 className="text-sm font-medium mb-2">Models ({expandedModels.length})</h4>
              {loadingModels ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading models...
                </div>
              ) : expandedModels.length === 0 ? (
                <p className="text-sm text-muted-foreground">No models available</p>
              ) : (
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {expandedModels.map((model) => (
                    <div key={model.upstream_id} className="p-2 border rounded-lg text-xs">
                      <div className="font-medium">{model.display_name || model.upstream_id}</div>
                      <div className="flex gap-1 mt-1 flex-wrap">
                        {model.supports_images && (
                          <Badge variant="secondary" className="text-[10px] px-1">Vision</Badge>
                        )}
                        {model.supports_reasoning && (
                          <Badge variant="secondary" className="text-[10px] px-1">Reasoning</Badge>
                        )}
                        {model.context_window && (
                          <span className="text-muted-foreground">
                            {model.context_window.toLocaleString()} ctx
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </TableCell>
        </TableRow>
      )}
    </>
  )
}

interface ProviderFormModalProps {
  providerId: string | null
  preset?: {
    id: string
    name: string
    shortcut?: string
    endpoint: string
    auth: "bearer" | "api-key"
    icon: string
  } | null
  providers: Provider[]
  onClose: () => void
  onSave: () => void
}

function ProviderFormModal({ providerId, preset, providers, onClose, onSave }: ProviderFormModalProps) {
  const { t } = useTranslation()

  // Get existing provider data for editing
  const existingProvider = providerId ? providers.find(p => p.id === providerId) : null

  // Initialize form with preset values if provided
  const [form, setForm] = useState(() => {
    if (preset) {
      return {
        id: preset.id,
        display_name: preset.name,
        endpoint_base: preset.endpoint,
        auth_type: preset.auth as "bearer" | "api-key",
        api_key: "",
      }
    }
    if (existingProvider) {
      return {
        id: existingProvider.id,
        display_name: existingProvider.display_name,
        endpoint_base: existingProvider.endpoint_base || existingProvider.base_url || "",
        auth_type: existingProvider.auth_type || "bearer",
        api_key: "",
      }
    }
    return {
      id: "",
      display_name: "",
      endpoint_base: "",
      auth_type: "bearer" as const,
      api_key: "",
    }
  })

  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!form.id.trim() || !form.endpoint_base.trim()) return

    setIsSaving(true)
    setError(null)
    setSuccess(false)
    try {
      if (providerId) {
        await api.providers.update(providerId, {
          display_name: form.display_name,
          endpoint_base: form.endpoint_base,
          auth_type: form.auth_type,
          api_key: form.api_key || undefined,
        })
      } else {
        await api.providers.create({
          id: form.id,
          display_name: form.display_name,
          endpoint_base: form.endpoint_base,
          auth_type: form.auth_type,
          api_key: form.api_key || undefined,
        })
      }
      setSuccess(true)
      onSave()
      setTimeout(onClose, 1000)
    } catch (err) {
      setError(String(err))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-[560px] max-h-[90vh] overflow-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>
              {providerId ? t("providers.editTitle", "Edit Provider") : t("providers.addTitle", "Add Provider")}
            </CardTitle>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            {!providerId && (
              <div className="space-y-2">
                <Label htmlFor="provider-id">Provider ID</Label>
                <Input
                  id="provider-id"
                  placeholder="e.g., openai"
                  value={form.id}
                  onChange={(e) => setForm({ ...form, id: e.target.value })}
                  required
                />
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="display-name">Display Name</Label>
              <Input
                id="display-name"
                placeholder="e.g., OpenAI"
                value={form.display_name}
                onChange={(e) => setForm({ ...form, display_name: e.target.value })}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="endpoint">Endpoint Base</Label>
              <Input
                id="endpoint"
                type="url"
                placeholder="https://api.openai.com/v1"
                value={form.endpoint_base}
                onChange={(e) => setForm({ ...form, endpoint_base: e.target.value })}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="auth-type">Auth Type</Label>
              <Select value={form.auth_type} onValueChange={(v) => setForm({ ...form, auth_type: v as "bearer" | "api-key" })}>
                <SelectTrigger id="auth-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="bearer">Bearer Token</SelectItem>
                  <SelectItem value="api-key">API Key Header</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="api-key">API Key</Label>
              <Input
                id="api-key"
                type="password"
                placeholder="sk-..."
                value={form.api_key}
                onChange={(e) => setForm({ ...form, api_key: e.target.value })}
                className="font-mono"
              />
            </div>

            {error && (
              <div className="text-sm text-destructive bg-destructive/10 p-2 rounded flex items-center gap-2">
                <AlertCircle className="h-4 w-4" />
                {error}
              </div>
            )}

            {success && (
              <div className="text-sm text-emerald-500 bg-emerald-500/10 p-2 rounded flex items-center gap-2">
                <Check className="h-4 w-4" />
                {t("providers.saved", "Saved successfully")}
              </div>
            )}

            <div className="flex gap-2 justify-end pt-4">
              <Button type="button" variant="outline" onClick={onClose}>
                {t("providers.cancel", "Cancel")}
              </Button>
              <Button type="submit" disabled={isSaving || success}>
                {isSaving ? t("providers.saving", "Saving...") : t("providers.save", "Save")}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}