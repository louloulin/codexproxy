import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { api } from "@/lib/api"
import { Loader2, Zap, CheckCircle, XCircle } from "lucide-react"
import type { CodexTarget, ProbeResult } from "@/types/codex"

interface ProviderBlockProps {
  targets: CodexTarget[]
  onApplied?: () => void
}

// 按 Provider 分组 targets
function groupTargetsByProvider(targets: CodexTarget[]): Record<string, CodexTarget[]> {
  return targets.reduce((acc, target) => {
    if (!acc[target.providerId]) {
      acc[target.providerId] = []
    }
    acc[target.providerId].push(target)
    return acc
  }, {} as Record<string, CodexTarget[]>)
}

/**
 * ProviderBlock - Provider/Model 选择和测试组件
 *
 * 拆分自 CodexPage.tsx，职责：
 * - 显示 Provider 和 Model 列表
 * - 测试连接
 * - 应用配置
 */
export function ProviderBlock({ targets, onApplied }: ProviderBlockProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const [selectedProvider, setSelectedProvider] = useState("")
  const [selectedModel, setSelectedModel] = useState("")
  const [probeResult, setProbeResult] = useState<ProbeResult | null>(null)
  const [probing, setProbing] = useState(false)
  const [testingAll, setTestingAll] = useState(false)
  const [probeResults, setProbeResults] = useState<Record<string, ProbeResult>>({})

  // 按 Provider 分组
  const groupedTargets = groupTargetsByProvider(targets)
  const providerIds = Object.keys(groupedTargets)

  const currentTarget = targets.find(t => t.providerId === selectedProvider && t.modelId === selectedModel)

  // 测试单个连接
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

  // 测试所有连接
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
        const errResult = { ok: false, latencyMs: 0, error: { code: "error", message: String(e) } }
        setProbeResults(prev => ({ ...prev, [key]: errResult }))
      }
    }))
    setTestingAll(false)
  }

  // 选择
  const handleSelect = (providerId: string, modelId: string) => {
    setSelectedProvider(providerId)
    setSelectedModel(modelId)
  }

  // 应用配置
  const handleApply = async () => {
    if (!selectedProvider || !selectedModel) return
    try {
      await api.codex.apply(selectedProvider, selectedModel)
      queryClient.invalidateQueries({ queryKey: ["codex-state"] })
      onApplied?.()
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
        {/* 测试全部按钮 */}
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleTestAll}
            disabled={testingAll || targets.length === 0}
          >
            {testingAll ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Zap className="mr-2 h-4 w-4" />}
            {testingAll ? t("provider.testingAll") : t("provider.testAll")}
          </Button>
          {Object.keys(probeResults).length > 0 && (
            <span className="text-sm text-muted-foreground">
              {`${Object.values(probeResults).filter(r => r.ok).length}/${Object.keys(probeResults).length} OK`}
            </span>
          )}
        </div>

        {/* 外部 Provider 警告 */}
        {targets.some(t => t.source !== "builtin") && (
          <Alert variant="default" className="border-yellow-300 bg-yellow-50">
            <AlertDescription className="text-yellow-800 text-sm">
              {t("provider.externalWarning")}
            </AlertDescription>
          </Alert>
        )}

        {/* Provider 分组列表 */}
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

        {/* 应用按钮 */}
        <div className="flex gap-2">
          <Button onClick={handleApply} disabled={!selectedProvider || !selectedModel}>
            {t("provider.applyCodex")}
          </Button>
        </div>

        {/* 选中目标信息 */}
        {currentTarget && (
          <div className="p-3 bg-muted rounded-lg">
            <p className="text-sm font-medium">{t("provider.targetUrl")}:</p>
            <p className="text-xs font-mono text-muted-foreground">{currentTarget.baseUrl}</p>
          </div>
        )}

        {/* 探测结果 */}
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
