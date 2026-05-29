import { useTranslation } from "react-i18next"
import { useQuery } from "@tanstack/react-query"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { cn } from "@/lib/utils"
import { api } from "@/lib/api"

// ProviderHealthRow is used by the API response
type ProviderHealthRow = {
  provider_id: string
  provider_name: string
  requests: number
  errors: number
  error_rate: number
  avg_latency_ms: number
  last_request_ts: number | null
}

type HealthLevel = "healthy" | "degraded" | "down" | "idle"

function healthLevel(r: ProviderHealthRow): HealthLevel {
  if (r.error_rate < 0 || r.requests === 0) return "idle"
  if (r.error_rate < 5) return "healthy"
  if (r.error_rate < 50) return "degraded"
  return "down"
}

function HealthDot({ level }: { level: HealthLevel }) {
  return (
    <span
      className={cn(
        "inline-block h-2 w-2 rounded-full",
        level === "healthy" && "bg-emerald-500",
        level === "degraded" && "bg-amber-500",
        level === "down" && "bg-red-500",
        level === "idle" && "bg-gray-400"
      )}
    />
  )
}

export function ProviderHealthCard() {
  const { t } = useTranslation()

  const { data, isLoading } = useQuery({
    queryKey: ["provider-health"],
    queryFn: async () => {
      try {
        const result = await api.stats.providerHealth()
        // Normalize rows to ensure all fields exist
        return {
          rows: (result?.rows ?? []).map((r: any) => ({
            provider_id: r.provider_id,
            provider_name: r.provider_name || r.provider_id,
            requests: r.requests || 0,
            errors: r.errors || 0,
            error_rate: r.error_rate || 0,
            avg_latency_ms: r.avg_latency_ms ?? r.latency_ms ?? 0,
            last_request_ts: r.last_request_ts ?? r.last_seen ?? null,
          }))
        }
      } catch {
        // Return null for unavailable API
        return null
      }
    },
    refetchInterval: 30000,
  })

  if (isLoading) {
    return <div className="animate-pulse space-y-2">
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="h-12 bg-muted rounded" />
      ))}
    </div>
  }

  if (!data?.rows || data.rows.length === 0) {
    return (
      <p className="text-muted-foreground text-center py-8">
        {t("providerHealth.empty", "No providers configured")}
      </p>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{t("providerHealth.provider", "Provider")}</TableHead>
          <TableHead className="text-right">{t("providerHealth.requests", "Requests")}</TableHead>
          <TableHead className="text-right">{t("providerHealth.errorRate", "Error Rate")}</TableHead>
          <TableHead className="text-right">{t("providerHealth.latency", "Latency")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.rows.map((row) => {
          const level = healthLevel(row)
          return (
            <TableRow key={row.provider_id}>
              <TableCell>
                <div className="flex items-center gap-2">
                  <HealthDot level={level} />
                  <span className="font-medium">{row.provider_name}</span>
                </div>
              </TableCell>
              <TableCell className="text-right font-mono">
                {row.requests.toLocaleString()}
              </TableCell>
              <TableCell className="text-right">
                <Badge
                  variant={
                    level === "healthy" ? "success" :
                    level === "degraded" ? "warning" :
                    level === "down" ? "destructive" : "secondary"
                  }
                >
                  {row.error_rate >= 0 ? `${row.error_rate.toFixed(1)}%` : "—"}
                </Badge>
              </TableCell>
              <TableCell className="text-right font-mono">
                {row.avg_latency_ms > 0 ? `${Math.round(row.avg_latency_ms)}ms` : "—"}
              </TableCell>
            </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}