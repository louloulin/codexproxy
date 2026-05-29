import { useTranslation } from "react-i18next"
import { useQuery } from "@tanstack/react-query"
import { Card, CardContent } from "@/components/ui/card"
import { api } from "@/lib/api"
import { Activity, Zap, TrendingUp, Clock, Hash } from "lucide-react"

type TimeRange = "24h" | "7d" | "30d"

function formatNumber(n: number | undefined | null): string {
  if (n === undefined || n === null) return "0"
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`
  return String(n)
}

function formatUptime(seconds: number | undefined): string {
  if (seconds === undefined || seconds === null) return "—"
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`
  return `${Math.floor(seconds / 86400)}d`
}

function formatMs(ms: number | undefined | null): string {
  if (ms === undefined || ms === null) return "—"
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`
  return `${Math.round(ms)}ms`
}

interface StatsCardProps {
  title: string
  value: string | number
  icon: React.ElementType
  subValue?: string
  variant?: "default" | "success" | "warning" | "danger"
}

function StatsCard({ title, value, icon: Icon, subValue, variant = "default" }: StatsCardProps) {
  const variantStyles = {
    default: "text-foreground",
    success: "text-emerald-600 dark:text-emerald-400",
    warning: "text-amber-600 dark:text-amber-400",
    danger: "text-red-600 dark:text-red-400",
  }

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <p className="text-sm font-medium text-muted-foreground">{title}</p>
            <p className={`text-3xl font-bold ${variantStyles[variant]}`}>
              {typeof value === "number" ? formatNumber(value) : value}
            </p>
            {subValue && (
              <p className="text-xs text-muted-foreground">{subValue}</p>
            )}
          </div>
          <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
            <Icon className="h-6 w-6 text-primary" />
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export function StatsCards({ range }: { range: TimeRange }) {
  const { t } = useTranslation()

  // Fetch detailed request stats
  const { data: statsData, isLoading: statsLoading } = useQuery({
    queryKey: ["requestStats", range],
    queryFn: () => api.stats.requestStats(range),
  })

  // Fetch basic stats for uptime
  const { data: basicStats } = useQuery({
    queryKey: ["stats", range],
    queryFn: () => api.stats.get(range),
  })

  // Fetch latency stats
  const { data: latencyData } = useQuery({
    queryKey: ["latencyStats", range],
    queryFn: () => api.stats.latencyStats(range),
  })

  // Calculate totals from request stats
  const totals = statsData?.rows?.reduce(
    (acc, row) => ({
      requests: acc.requests + row.requests,
      errors: acc.errors + row.errors,
      tokens: acc.tokens + row.total_tokens,
      promptTokens: acc.promptTokens + row.prompt_tokens,
    }),
    { requests: 0, errors: 0, tokens: 0, promptTokens: 0 }
  ) ?? { requests: 0, errors: 0, tokens: 0, promptTokens: 0 }

  // Calculate error rate
  const errorRate = totals.requests > 0
    ? (totals.errors / totals.requests) * 100
    : 0

  // Get latency data - latencyStats can have avgMs or be empty
  const avgLatency = (latencyData as any)?.stats?.avgMs ?? (latencyData as any)?.stats?.minMs ?? 0
  const p95Latency = (latencyData as any)?.stats?.maxMs ?? 0

  if (statsLoading || !statsData) {
    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
        {Array.from({ length: 5 }).map((_, i) => (
          <Card key={i}>
            <CardContent className="p-6">
              <div className="animate-pulse space-y-3">
                <div className="h-4 w-20 bg-muted rounded" />
                <div className="h-8 w-16 bg-muted rounded" />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    )
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
      <StatsCard
        title={t("stats.requests", "Requests")}
        value={totals.requests}
        icon={Activity}
        subValue={t("stats.requestsSub", `in last ${range}`)}
      />
      <StatsCard
        title={t("stats.errors", "Errors")}
        value={totals.errors}
        icon={Zap}
        subValue={errorRate > 0 ? `${errorRate.toFixed(1)}% error rate` : "no errors"}
        variant={errorRate > 0 ? (errorRate > 5 ? "danger" : "warning") : "success"}
      />
      <StatsCard
        title={t("stats.tokens", "Tokens")}
        value={totals.tokens}
        icon={Hash}
        subValue={t("stats.tokensSub", "total consumed")}
      />
      <StatsCard
        title={t("stats.latency", "Latency")}
        value={formatMs(avgLatency)}
        icon={Clock}
        subValue={t("stats.latencySub", `P95: ${formatMs(p95Latency)}`)}
      />
      <StatsCard
        title={t("stats.uptime", "Uptime")}
        value={formatUptime(basicStats?.uptime_seconds)}
        icon={TrendingUp}
        subValue={t("stats.uptimeSub", "since last restart")}
      />
    </div>
  )
}
