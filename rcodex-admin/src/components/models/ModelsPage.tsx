import { useTranslation } from "react-i18next"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useState } from "react"
import { Plus, Trash2, Calendar, X } from "lucide-react"
import { api } from "@/lib/api"
import { PageTour, type TourStep } from "@/components/PageTour"

// Models PageTour steps
const MODELS_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='models-switcher']",
    title: "Provider Switcher",
    description: "Switch between providers to manage models for each one. Only built-in providers (mimo, deepseek) support model management here.",
    placement: "bottom",
  },
  {
    target: "[data-tour='models-add-form']",
    title: "Add Model",
    description: "Add new models by specifying the upstream ID and optional display name. Built-in models cannot be modified.",
    placement: "bottom",
  },
  {
    target: "[data-tour='models-table']",
    title: "Models Table",
    description: "View all models for the selected provider. Set deprecated dates for custom models or delete them.",
    placement: "top",
  },
]

const BUILTIN_PROVIDER_IDS = ["mimo", "deepseek"]

type Model = {
  id: number
  upstream_id: string
  display_name: string | null
  context_window: number | null
  supports_images: boolean
  supports_reasoning: boolean
  supports_web_search: boolean
  is_builtin: boolean
  deprecated_after: string | null
}

function formatContext(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(0)}M`
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(0)}k`
  return String(tokens)
}

function DeprecatedDateModal({
  model,
  onClose,
  onSave,
  isSaving,
}: {
  model: Model
  onClose: () => void
  onSave: (date: string) => void
  isSaving: boolean
}) {
  const { t } = useTranslation()
  const [date, setDate] = useState(model.deprecated_after || "")

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-96">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>{t("models.setDeprecated", "Set Deprecated Date")}</CardTitle>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            {t("models.deprecatedDesc", "Set a date after which this model will be considered deprecated.")}
          </p>
          <div>
            <label className="text-sm font-medium block mb-1">{t("models.deprecatedDate", "Deprecated Date")}</label>
            <Input
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </div>
          {date && (
            <p className="text-xs text-muted-foreground">
              {t("models.clearDate", "Leave empty to clear the deprecated date.")}
            </p>
          )}
          <div className="flex gap-2 justify-end">
            <Button variant="outline" onClick={onClose}>
              {t("models.cancel", "Cancel")}
            </Button>
            <Button
              onClick={() => onSave(date)}
              disabled={isSaving}
            >
              {isSaving ? t("models.saving", "Saving...") : t("models.save", "Save")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

export function ModelsPage() {
  const { t } = useTranslation()
  const [activeProvider, setActiveProvider] = useState("mimo")
  const [newModel, setNewModel] = useState({ upstream_id: "", display_name: "" })
  const [editingModel, setEditingModel] = useState<Model | null>(null)
  const queryClient = useQueryClient()

  const { data, isLoading } = useQuery({
    queryKey: ["models", activeProvider],
    queryFn: () => api.models.list(activeProvider),
    enabled: BUILTIN_PROVIDER_IDS.includes(activeProvider),
  })

  const createMutation = useMutation({
    mutationFn: (modelData: { upstream_id: string; display_name?: string }) =>
      api.models.create(activeProvider, modelData),
    onSuccess: () => {
      setNewModel({ upstream_id: "", display_name: "" })
      queryClient.invalidateQueries({ queryKey: ["models", activeProvider] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (id: number) => api.models.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["models", activeProvider] })
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: { display_name?: string; deprecated_after?: string | null } }) =>
      api.models.update(id, data),
    onSuccess: () => {
      setEditingModel(null)
      queryClient.invalidateQueries({ queryKey: ["models", activeProvider] })
    },
  })

  function addModel() {
    if (!newModel.upstream_id.trim()) return
    createMutation.mutate({
      upstream_id: newModel.upstream_id,
      display_name: newModel.display_name || undefined,
    })
  }

  function saveDeprecatedDate(model: Model, date: string) {
    updateMutation.mutate({
      id: model.id,
      data: { deprecated_after: date || null },
    })
  }

  const models = data?.models ?? []

  return (
    <PageTour pageKey="models" steps={MODELS_TOUR_STEPS}>
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("models.title", "Models")}</h1>
        <p className="text-muted-foreground">
          {t("models.subtitle", "Manage AI models for each provider")}
        </p>
      </div>

      {/* Provider Switcher */}
      <div className="flex gap-2" data-tour="models-switcher">
        {BUILTIN_PROVIDER_IDS.map((provider) => (
          <Button
            key={provider}
            variant={activeProvider === provider ? "default" : "outline"}
            onClick={() => setActiveProvider(provider)}
          >
            {provider}
          </Button>
        ))}
      </div>

      {/* Models Table */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>{t("models.title", "Models")}</CardTitle>
            <CardDescription>
              {t("models.desc", "Configure model settings for this provider")}
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          {/* Add Model Form */}
          <div className="flex gap-2 mb-4" data-tour="models-add-form">
            <Input
              placeholder={t("models.upstreamIdPlaceholder", "Upstream ID (e.g., gpt-4)")}
              value={newModel.upstream_id}
              onChange={(e) => setNewModel({ ...newModel, upstream_id: e.target.value })}
              className="max-w-xs"
              disabled={createMutation.isPending}
            />
            <Input
              placeholder={t("models.displayNamePlaceholder", "Display name (optional)")}
              value={newModel.display_name}
              onChange={(e) => setNewModel({ ...newModel, display_name: e.target.value })}
              className="max-w-xs"
              disabled={createMutation.isPending}
            />
            <Button onClick={addModel} disabled={createMutation.isPending}>
              <Plus className="h-4 w-4 mr-1" />
              {createMutation.isPending ? t("models.adding", "Adding...") : t("models.add", "Add")}
            </Button>
          </div>

          {/* Models Table */}
          <Table data-tour="models-table">
            <TableHeader>
              <TableRow>
                <TableHead>{t("models.upstreamId", "Upstream ID")}</TableHead>
                <TableHead>{t("models.displayName", "Display Name")}</TableHead>
                <TableHead>{t("models.capabilities", "Capabilities")}</TableHead>
                <TableHead className="text-right">{t("models.context", "Context Window")}</TableHead>
                <TableHead className="text-right">{t("models.actions", "Actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={5} className="text-center text-muted-foreground py-8">
                    {t("models.loading", "Loading...")}
                  </TableCell>
                </TableRow>
              ) : models.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} className="text-center text-muted-foreground py-8">
                    {t("models.empty", "No models configured")}
                  </TableCell>
                </TableRow>
              ) : (
                models.map((model) => (
                  <TableRow key={model.id}>
                    <TableCell>
                      <code className="text-sm">{model.upstream_id}</code>
                      {model.is_builtin && (
                        <Badge variant="secondary" className="ml-2 text-xs">Built-in</Badge>
                      )}
                    </TableCell>
                    <TableCell>{model.display_name || "—"}</TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        {model.supports_images && (
                          <Badge variant="outline" className="text-xs">
                            {t("models.vision", "Vision")}
                          </Badge>
                        )}
                        {model.supports_reasoning && (
                          <Badge variant="outline" className="text-xs">
                            {t("models.reasoning", "Reasoning")}
                          </Badge>
                        )}
                        {model.supports_web_search && (
                          <Badge variant="outline" className="text-xs">
                            {t("models.webSearch", "Web Search")}
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right text-muted-foreground">
                      {model.context_window ? formatContext(model.context_window) : "—"}
                    </TableCell>
                    <TableCell className="text-right">
                      {model.deprecated_after && !model.is_builtin && (
                        <Badge variant="destructive" className="mr-2 text-xs">
                          <Calendar className="h-3 w-3 mr-1" />
                          {new Date(model.deprecated_after).toLocaleDateString()}
                        </Badge>
                      )}
                      {!model.is_builtin && (
                        <Button
                          size="sm"
                          variant="ghost"
                          className="text-muted-foreground"
                          onClick={() => setEditingModel(model)}
                          title="Edit deprecated date"
                        >
                          <Calendar className="h-4 w-4" />
                        </Button>
                      )}
                      {!model.is_builtin && (
                        <Button
                          size="sm"
                          variant="ghost"
                          className="text-destructive"
                          onClick={() => deleteMutation.mutate(model.id)}
                          disabled={deleteMutation.isPending}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Deprecated Date Modal */}
      {editingModel && (
        <DeprecatedDateModal
          model={editingModel}
          onClose={() => setEditingModel(null)}
          onSave={(date) => saveDeprecatedDate(editingModel, date)}
          isSaving={updateMutation.isPending}
        />
      )}
    </div>
    </PageTour>
  )
}