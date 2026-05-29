import { useTranslation } from "react-i18next"
import { useQuery } from "@tanstack/react-query"
import { Link } from "react-router-dom"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { api } from "@/lib/api"
import { ExternalLink } from "lucide-react"
import { cn } from "@/lib/utils"

// Type for log rows returned by the API
type LogRow = {
  id: number
  ts: number
  provider_id: string
  client_model: string
  upstream_model: string
  endpoint: string
  status_code: number
  duration_ms: number
  prompt_tokens: number | null
  completion_tokens: number | null
  total_tokens: number | null
  stream: boolean
  error_code: string | null
  error_snippet: string | null
}

export function RecentLogsTable() {
  const { t } = useTranslation()

  const { data, isLoading } = useQuery({
    queryKey: ["recent-logs"],
    queryFn: async () => {
      try {
        return await api.logs.list({ limit: 10 })
      } catch {
        // Return null for unavailable API
        return null
      }
    },
    refetchInterval: 30000,
  })

  if (isLoading) {
    return (
      <div className="animate-pulse space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-10 bg-muted rounded" />
        ))}
      </div>
    )
  }

  if (!data?.logs || data.logs.length === 0) {
    return (
      <p className="text-muted-foreground text-center py-8">
        {t("recentLogs.empty", "No recent logs")}
      </p>
    )
  }

  return (
    <div className="space-y-2">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t("recentLogs.time", "Time")}</TableHead>
            <TableHead>{t("recentLogs.provider", "Provider")}</TableHead>
            <TableHead>{t("recentLogs.model", "Model")}</TableHead>
            <TableHead className="text-right">{t("recentLogs.status", "Status")}</TableHead>
            <TableHead className="text-right">{t("recentLogs.duration", "Duration")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.logs.slice(0, 5).map((row: LogRow) => (
            <TableRow key={row.id} className="cursor-pointer hover:bg-muted/50">
              <TableCell className="text-muted-foreground text-xs whitespace-nowrap">
                {new Date(row.ts).toLocaleTimeString()}
              </TableCell>
              <TableCell>
                <Badge variant="outline">{row.provider_id}</Badge>
              </TableCell>
              <TableCell>
                <Link to={`/logs?highlight=${row.id}`} className="hover:underline">
                  <code className="text-xs">{row.client_model}</code>
                </Link>
              </TableCell>
              <TableCell className="text-right">
                <Badge
                  variant={row.status_code >= 400 ? "destructive" : "success"}
                  className={cn(
                    row.status_code >= 400 && "text-red-600",
                    row.status_code >= 500 && "bg-red-100 text-red-800"
                  )}
                >
                  {row.status_code}
                </Badge>
              </TableCell>
              <TableCell className="text-right text-muted-foreground text-xs">
                {row.duration_ms}ms
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      <div className="flex justify-end">
        <Button variant="ghost" size="sm" asChild>
          <Link to="/logs">
            {t("recentLogs.viewAll", "View all logs")}
            <ExternalLink className="h-3 w-3 ml-1" />
          </Link>
        </Button>
      </div>
    </div>
  )
}