import { useTranslation } from "react-i18next"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { api } from "@/lib/api"
import { RotateCcw, Trash2 } from "lucide-react"
import type { BackupPair } from "@/types/codex"

interface BackupCardProps {
  pairs: BackupPair[]
}

/**
 * BackupCard - 备份历史卡片组件
 *
 * 拆分自 CodexPage.tsx，职责：
 * - 显示备份历史列表
 * - 恢复备份
 * - 删除备份
 */
export function BackupCard({ pairs }: BackupCardProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  // 恢复备份
  const restoreMutation = useMutation({
    mutationFn: async (ts: number) => api.codex.restore(ts),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["codex-state"] }),
  })

  // 删除备份
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
                    {pair.preserved
                      ? <Badge variant="success">{t("backup.preserved")}</Badge>
                      : <Badge variant="outline">{t("backup.snapshot")}</Badge>}
                  </TableCell>
                  <TableCell>
                    <code className="text-xs">
                      {pair.provider ?? "?"} / {pair.model ?? "?"}
                    </code>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-2">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => restoreMutation.mutate(pair.ts)}
                      >
                        <RotateCcw className="mr-1 h-4 w-4" /> {t("backup.restore")}
                      </Button>
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={() => {
                          if (confirm(pair.preserved
                            ? t("backup.confirmDeletePreserved")
                            : t("backup.confirmDelete"))) {
                            deleteMutation.mutate(pair.ts)
                          }
                        }}
                      >
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
