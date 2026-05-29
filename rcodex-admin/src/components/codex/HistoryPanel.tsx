import { useQuery } from "@tanstack/react-query"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Loader2, RefreshCw, Download } from "lucide-react"
import type { CodexHistoryEntry } from "@/types/codex"

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

export function HistoryPanel() {
  const { t } = useTranslation()
  const { data: history, isLoading, refetch } = useQuery({
    queryKey: ["codex-history"],
    queryFn: () => api.codex.history(),
  })

  const downloadBundle = async (id: number) => {
    try {
      const result = await api.codex.historyBundle(id)
      if (result.ok && result.data) {
        const { files, scripts } = result.data
        downloadBlob(files.auth_json, "auth.json", "application/json")
        downloadBlob(files.config_toml, "config.toml", "text/plain")
        downloadBlob(scripts.posix, `apply-${id}.sh`, "text/plain")
        downloadBlob(scripts.powershell, `apply-${id}.ps1`, "text/plain")
      }
    } catch (e) {
      console.error("Failed to download bundle:", e)
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>{t("history.title")}</CardTitle>
            <CardDescription>{t("history.description")}</CardDescription>
          </div>
          <Button variant="outline" size="icon" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin" />
          </div>
        ) : history?.ok && history.data && history.data.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("history.time")}</TableHead>
                <TableHead>{t("history.kind")}</TableHead>
                <TableHead>{t("history.provider")}</TableHead>
                <TableHead>{t("history.model")}</TableHead>
                <TableHead>{t("history.note")}</TableHead>
                <TableHead className="text-right">{t("history.actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {history.data.map((entry: CodexHistoryEntry) => (
                <TableRow key={entry.id}>
                  <TableCell className="font-mono text-sm">
                    {new Date(entry.createdAt).toLocaleString()}
                  </TableCell>
                  <TableCell>
                    <Badge variant={entry.kind === "Apply" ? "default" : entry.kind === "Restore" ? "secondary" : "outline"}>
                      {entry.kind}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {(entry as any).provider_id ?? "-"}
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {(entry as any).model_id ?? "-"}
                  </TableCell>
                  <TableCell className="text-muted-foreground">{entry.note ?? ""}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button size="sm" variant="ghost" onClick={() => downloadBundle(entry.id)} title={t("history.download")}>
                        <Download className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <div className="text-center py-8 text-muted-foreground">
            <p>{t("history.noHistory")}</p>
            <p className="text-sm">{t("history.noHistoryHint")}</p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
