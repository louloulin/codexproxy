import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { CheckCircle, XCircle, Upload, Download, Info, Key, ArrowRight, ArrowLeft } from "lucide-react"

interface ImportModalProps {
  open: boolean
  onClose: () => void
  onImported: () => void
}

export function ImportModal({ open, onClose, onImported }: ImportModalProps) {
  const { t } = useTranslation()
  const [phase, setPhase] = useState<"guide" | "form">("guide")
  const [authJson, setAuthJson] = useState("")
  const [configToml, setConfigToml] = useState("")
  const [providerId, setProviderId] = useState("")
  const [modelId, setModelId] = useState("")
  const [note, setNote] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  // Reset when modal opens
  useEffect(() => {
    if (open) {
      setPhase("guide")
      setAuthJson("")
      setConfigToml("")
      setProviderId("")
      setModelId("")
      setNote("")
      setBusy(false)
      setError(null)
      setSuccess(false)
    }
  }, [open])

  if (!open) return null

  async function handleSubmit() {
    // Validate JSON
    try {
      JSON.parse(authJson)
    } catch {
      setError(t("import.errorInvalidJson"))
      return
    }

    if (!configToml.trim()) {
      setError(t("import.errorConfigRequired"))
      return
    }

    setBusy(true)
    setError(null)

    try {
      const result = await api.codex.import({
        authJson,
        configToml,
        providerId: providerId || undefined,
        modelId: modelId || undefined,
        note: note || undefined,
      })

      if (result.ok) {
        setSuccess(true)
        setTimeout(() => {
          onImported()
          onClose()
        }, 1500)
      } else {
        setError(result.error || "Import failed")
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Upload className="h-5 w-5" />
              <CardTitle>
                {phase === "guide" ? t("import.guideTitle") : t("import.title")}
              </CardTitle>
            </div>
            <Button variant="ghost" size="sm" onClick={onClose}>×</Button>
          </div>
          <CardDescription>
            {phase === "guide"
              ? t("import.guideDesc")
              : t("import.configTomlPlaceholder")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <XCircle className="h-4 w-4" />
              <AlertTitle>{t("common.error")}</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {success && (
            <Alert variant="default" className="border-green-500 bg-green-50">
              <CheckCircle className="h-4 w-4 text-green-600" />
              <AlertTitle className="text-green-800">{t("common.success")}</AlertTitle>
              <AlertDescription className="text-green-700">
                {t("import.guideDesc")}
              </AlertDescription>
            </Alert>
          )}

          {phase === "guide" ? (
            <div className="space-y-4">
              <div>
                <h3 className="font-medium mb-2">{t("import.title")}</h3>
                <p className="text-sm text-muted-foreground mb-4">
                  {t("import.guideDesc")}
                </p>
              </div>

              <Alert>
                <Info className="h-4 w-4" />
                <AlertTitle>{t("setup.authJson")}</AlertTitle>
                <AlertDescription className="space-y-2 mt-2">
                  <div>
                    <p className="font-mono text-xs">macOS/Linux:</p>
                    <code className="text-xs">~/.codex/auth.json</code>
                  </div>
                  <div>
                    <p className="font-mono text-xs">Windows:</p>
                    <code className="text-xs">%USERPROFILE%\.codex\auth.json</code>
                  </div>
                </AlertDescription>
              </Alert>

              <Alert variant="default" className="border-yellow-500 bg-yellow-50">
                <Key className="h-4 w-4 text-yellow-600" />
                <AlertTitle className="text-yellow-800">{t("import.warning")}</AlertTitle>
                <AlertDescription className="text-yellow-700 text-sm">
                  {t("import.guideDesc")}
                </AlertDescription>
              </Alert>

              <div className="flex justify-end gap-2 pt-4">
                <Button variant="outline" onClick={onClose}>{t("import.cancel")}</Button>
                <Button onClick={() => setPhase("form")}>
                  {t("import.import")} <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="authJson">{t("import.authJson")} *</Label>
                <textarea
                  id="authJson"
                  value={authJson}
                  onChange={(e) => setAuthJson(e.target.value)}
                  placeholder={t("import.authJsonPlaceholder")}
                  className="flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="configToml">{t("import.configToml")} *</Label>
                <textarea
                  id="configToml"
                  value={configToml}
                  onChange={(e) => setConfigToml(e.target.value)}
                  placeholder={t("import.configTomlPlaceholder")}
                  className="flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="providerId">{t("import.providerId")}</Label>
                  <Input
                    id="providerId"
                    value={providerId}
                    onChange={(e) => setProviderId(e.target.value)}
                    placeholder="minimax"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="modelId">{t("import.modelId")}</Label>
                  <Input
                    id="modelId"
                    value={modelId}
                    onChange={(e) => setModelId(e.target.value)}
                    placeholder="MiniMax-M2.7"
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="note">{t("import.note")}</Label>
                <Input
                  id="note"
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  placeholder="Import from backup 2026-01-15"
                />
              </div>

              <div className="flex justify-between gap-2 pt-4">
                <Button variant="outline" onClick={() => setPhase("guide")} disabled={busy}>
                  <ArrowLeft className="mr-2 h-4 w-4" /> Back
                </Button>
                <Button onClick={handleSubmit} disabled={busy || !authJson || !configToml}>
                  {busy ? t("import.importing") : t("import.import")}
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}

interface ExportModalProps {
  open: boolean
  onClose: () => void
}

export function ExportModal({ open, onClose }: ExportModalProps) {
  const { t } = useTranslation()
  const [downloading, setDownloading] = useState(false)
  const [downloaded, setDownloaded] = useState(false)

  function downloadBlob(content: string, filename: string, mime = "text/plain") {
    const blob = new Blob([content], { type: mime })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  async function handleExport() {
    setDownloading(true)
    try {
      const result = await api.codex.currentBundle()
      if (result.ok && result.data) {
        const tag = `${result.data.history.provider_id || "config"}-${result.data.history.model_id || "current"}`
        downloadBlob(result.data.files.auth_json, "auth.json", "application/json")
        downloadBlob(result.data.files.config_toml, "config.toml", "text/plain")
        downloadBlob(result.data.scripts.posix, `apply-${tag}.sh`, "text/x-shellscript")
        downloadBlob(result.data.scripts.powershell, `apply-${tag}.ps1`, "text/x-powershell")
        setDownloaded(true)
      }
    } catch (err) {
      console.error("Export failed:", err)
    } finally {
      setDownloading(false)
    }
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Download className="h-5 w-5" />
              <CardTitle>
                {downloaded ? t("common.success") : t("export.title")}
              </CardTitle>
            </div>
            <Button variant="ghost" size="sm" onClick={onClose}>×</Button>
          </div>
          <CardDescription>
            {t("export.desc")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {downloaded ? (
            <Alert variant="default" className="border-green-500 bg-green-50">
              <CheckCircle className="h-4 w-4 text-green-600" />
              <AlertTitle className="text-green-800">{t("common.success")}</AlertTitle>
              <AlertDescription className="text-green-700">
                {t("export.desc")}
              </AlertDescription>
            </Alert>
          ) : (
            <>
              <p className="text-sm text-muted-foreground">
                {t("export.desc")}
              </p>
              <ul className="space-y-2 text-sm">
                <li className="flex items-center gap-2">
                  <code className="bg-muted px-1 rounded">auth.json</code>
                  <span className="text-muted-foreground">{t("export.downloadAuth")}</span>
                </li>
                <li className="flex items-center gap-2">
                  <code className="bg-muted px-1 rounded">config.toml</code>
                  <span className="text-muted-foreground">{t("export.downloadConfig")}</span>
                </li>
                <li className="flex items-center gap-2">
                  <code className="bg-muted px-1 rounded">apply-*.sh</code>
                  <span className="text-muted-foreground">{t("export.posix")}</span>
                </li>
                <li className="flex items-center gap-2">
                  <code className="bg-muted px-1 rounded">apply-*.ps1</code>
                  <span className="text-muted-foreground">{t("export.powershell")}</span>
                </li>
              </ul>

              <Alert>
                <Key className="h-4 w-4" />
                <AlertTitle>{t("export.warning")}</AlertTitle>
                <AlertDescription className="text-sm">
                  {t("export.warning")}
                </AlertDescription>
              </Alert>
            </>
          )}

          <div className="flex justify-end gap-2 pt-4">
            <Button variant="outline" onClick={onClose}>
              {downloaded ? t("export.close") : t("common.cancel")}
            </Button>
            {!downloaded && (
              <Button onClick={handleExport} disabled={downloading}>
                {downloading ? t("common.loading") : t("export.downloadScript")}
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
