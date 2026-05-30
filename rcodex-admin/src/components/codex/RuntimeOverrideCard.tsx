import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { api } from "@/lib/api"

/**
 * RuntimeOverrideCard - 运行时覆盖组件
 *
 * 拆分自 CodexPage.tsx，职责：
 * - 显示当前运行时覆盖状态
 * - 设置/清除运行时覆盖
 */
export function RuntimeOverrideCard() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const [providerId, setProviderId] = useState("")
  const [modelId, setModelId] = useState("")

  // 获取当前覆盖状态
  const { data: override } = useQuery({
    queryKey: ["active-override"],
    queryFn: () => api.codex.activeOverride(),
  })

  // 设置覆盖
  const setMutation = useMutation({
    mutationFn: async () => api.codex.setOverride(providerId, modelId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["active-override"] }),
  })

  // 清除覆盖
  const clearMutation = useMutation({
    mutationFn: async () => api.codex.clearOverride(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["active-override"] }),
  })

  return (
    <Card data-tour="codex-override">
      <CardHeader>
        <CardTitle>{t("override.title")}</CardTitle>
        <CardDescription>{t("override.description")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 当前覆盖状态 */}
        <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
          <div>
            {override?.data ? (
              <div className="flex items-center gap-2">
                <Badge variant="default">{override.data.providerId}</Badge>
                <span>/</span>
                <Badge variant="outline">{override.data.modelId}</Badge>
              </div>
            ) : (
              <p className="text-muted-foreground">{t("override.noOverride")}</p>
            )}
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => clearMutation.mutate()}
            disabled={!override?.data || clearMutation.isPending}
          >
            {t("override.clear")}
          </Button>
        </div>

        {/* 设置覆盖表单 */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="text-sm font-medium">{t("provider.selectProvider")}</label>
            <input
              placeholder={t("override.providerPlaceholder")}
              value={providerId}
              onChange={(e) => setProviderId(e.target.value)}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm mt-1"
            />
          </div>
          <div>
            <label className="text-sm font-medium">{t("provider.selectModel")}</label>
            <input
              placeholder={t("override.modelPlaceholder")}
              value={modelId}
              onChange={(e) => setModelId(e.target.value)}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm mt-1"
            />
          </div>
        </div>

        <Button
          onClick={() => setMutation.mutate()}
          disabled={!providerId || !modelId || setMutation.isPending}
        >
          {t("override.set")}
        </Button>
      </CardContent>
    </Card>
  )
}
