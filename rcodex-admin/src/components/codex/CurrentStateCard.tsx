import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { api } from "@/lib/api"
import { Download, RefreshCw } from "lucide-react"
import type { CodexState } from "@/types/codex"

interface CurrentStateCardProps {
  state: CodexState | undefined
  onRefresh: () => void
}

// Download blob helper function
function downloadBlob(content: string, filename: string, mime = "text/plain"): void {
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

/**
 * CurrentStateCard - 显示当前Codex配置状态的卡片组件
 *
 * 拆分自 CodexPage.tsx，职责：
 * - 显示 Codex 目录、认证状态、配置状态
 * - 导出配置文件
 * - 编辑 Codex 目录
 */
export function CurrentStateCard({ state, onRefresh }: CurrentStateCardProps) {
  const { t } = useTranslation()
  const [editingDir, setEditingDir] = useState(false)
  const [dirInput, setDirInput] = useState("")

  // 解析 config.toml 中的 model 和 provider
  const parseModelProvider = (text: string | null) => {
    if (!text) return { model: null, provider: null }
    const modelMatch = /^\s*model\s*=\s*"([^"\n]+)"/m.exec(text)
    const providerMatch = /^\s*model_provider\s*=\s*"([^"\n]+)"/m.exec(text)
    return { model: modelMatch?.[1] ?? null, provider: providerMatch?.[1] ?? null }
  }

  const parsed = state?.configTomlText ? parseModelProvider(state.configTomlText) : { model: null, provider: null }

  // 认证所有者样式
  const ownerVariant = state?.authJsonOwner === "mimo2codex"
    ? "success"
    : state?.authJsonOwner === "external"
      ? "warning"
      : "secondary"
  const ownerLabel = state?.authJsonOwner === "mimo2codex"
    ? t("auth.mimo2codex")
    : state?.authJsonOwner === "external"
      ? t("auth.external")
      : t("auth.missing")

  // 导出配置文件
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

  // 设置 Codex 目录
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

  // 清除 Codex 目录
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
          {/* Codex 目录 */}
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.codexDir")}</p>
            {editingDir ? (
              <div className="flex gap-2 mt-1 flex-wrap">
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

          {/* 认证所有者 */}
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.authOwner")}</p>
            <Badge variant={ownerVariant as any} className="mt-1">{ownerLabel}</Badge>
            <p className="text-xs text-muted-foreground mt-1 font-mono truncate">{state?.authPath ?? "-"}</p>
          </div>

          {/* Provider */}
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.provider")}</p>
            <Badge variant="outline">{parsed.provider ?? "-"}</Badge>
          </div>

          {/* Model */}
          <div>
            <p className="text-sm text-muted-foreground">{t("codexState.model")}</p>
            <Badge variant="secondary">{parsed.model ?? "-"}</Badge>
          </div>
        </div>

        {/* 状态概览 */}
        <div className="flex gap-4 flex-wrap">
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
        </div>
      </CardContent>
    </Card>
  )
}
