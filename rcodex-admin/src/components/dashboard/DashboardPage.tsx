import { useTranslation } from "react-i18next"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Skeleton, SkeletonCard, SkeletonTable, SkeletonChart } from "@/components/ui/skeleton"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { useState, useEffect, useCallback, useRef } from "react"
import { RefreshCw, ExternalLink, Clock } from "lucide-react"
import { TokenChart } from "./TokenChart"
import { SetupBanner } from "./SetupBanner"
import { RecentLogsTable } from "./RecentLogsTable"
import { StatsCards } from "./StatsCards"
import { ProviderHealthCard } from "./ProviderHealthCard"
import { PageTour, type TourStep } from "@/components/PageTour"
import { api } from "@/lib/api"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

// Dashboard PageTour steps
const DASHBOARD_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='dashboard-range']",
    title: "Time Range",
    description: "Select the time range for viewing usage statistics. Options include 24 hours, 7 days, and 30 days.",
    placement: "bottom",
  },
  {
    target: "[data-tour='dashboard-stats']",
    title: "Usage Statistics",
    description: "View key metrics: total requests, errors, tokens used, cache hit rate, and latency percentiles.",
    placement: "bottom",
  },
  {
    target: "[data-tour='dashboard-chart']",
    title: "Token Usage Chart",
    description: "Visualize token consumption over time. Use bucket selectors to change granularity (hourly/daily).",
    placement: "top",
  },
  {
    target: "[data-tour='dashboard-health']",
    title: "Provider Health",
    description: "Monitor the health status of your configured providers. Green means healthy, yellow means degraded, red means down.",
    placement: "top",
  },
  {
    target: "[data-tour='dashboard-logs']",
    title: "Recent Logs",
    description: "Quick view of the most recent API requests. Click 'View All' to access the full logs page.",
    placement: "left",
  },
]

type TimeRange = "24h" | "7d" | "30d"
type TimeseriesBucket = "hour" | "day"

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`
  return String(n)
}

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`
  return `${Math.round(ms)}ms`
}

function formatTokens(n: number): string {
  if (n < 1000) return String(n)
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`
  return `${(n / 1_000_000).toFixed(2)}M`
}

export function DashboardPage() {
  const { t } = useTranslation()
  const [range, setRange] = useState<TimeRange>("24h")
  const [bucket, setBucket] = useState<TimeseriesBucket>("hour")
  const [loading, setLoading] = useState(true)
  const [statsData, setStatsData] = useState<any>(null)
  const [errorStatsData, setErrorStatsData] = useState<any>(null)
  const [latencyStatsData, setLatencyStatsData] = useState<any>(null)
  const lastPayloadRef = useRef({ stats: "", errorStats: "", latencyStats: "" })

  const loadData = useCallback(async () => {
    try {
      // Fetch each API independently to handle partial failures gracefully
      let stats = null
      let errorStats = null
      let latencyStats = null

      try {
        stats = await api.stats.requestStats(range)
      } catch (err) {
        console.warn("Request Stats API not available:", err)
      }

      try {
        errorStats = await api.stats.errorStats(range)
      } catch (err) {
        console.warn("Error stats API not available:", err)
      }

      try {
        latencyStats = await api.stats.latencyStats(range)
      } catch (err) {
        console.warn("Latency stats API not available:", err)
      }

      // Only update state if data changed (prevents flicker)
      const statsStr = JSON.stringify(stats)
      const errorStatsStr = JSON.stringify(errorStats)
      const latencyStatsStr = JSON.stringify(latencyStats)

      if (statsStr !== lastPayloadRef.current.stats) {
        lastPayloadRef.current.stats = statsStr
        setStatsData(stats)
      }
      if (errorStatsStr !== lastPayloadRef.current.errorStats) {
        lastPayloadRef.current.errorStats = errorStatsStr
        setErrorStatsData(errorStats)
      }
      if (latencyStatsStr !== lastPayloadRef.current.latencyStats) {
        lastPayloadRef.current.latencyStats = latencyStatsStr
        setLatencyStatsData(latencyStats)
      }
      setLoading(false)
    } catch (err) {
      console.error("Failed to load dashboard data:", err)
      setLoading(false)
    }
  }, [range])

  useEffect(() => {
    loadData()

    // Auto-refresh: 5s when visible, 30s when hidden
    let tid: ReturnType<typeof setInterval>
    function schedule() {
      if (tid) clearInterval(tid)
      const ms = document.visibilityState === "visible" ? 5000 : 30000
      tid = setInterval(loadData, ms)
    }
    schedule()
    document.addEventListener("visibilitychange", schedule)
    return () => {
      clearInterval(tid)
      document.removeEventListener("visibilitychange", schedule)
    }
  }, [loadData])

  return (
    <PageTour pageKey="dashboard" steps={DASHBOARD_TOUR_STEPS}>
      <div className="space-y-6">
        {/* Page Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{t("dashboard.title", "Dashboard")}</h1>
            <p className="text-muted-foreground">
              {t("dashboard.subtitle", "Overview of your API usage and system health")}
            </p>
          </div>
          <div className="flex items-center gap-2" data-tour="dashboard-range">
            <div className="flex gap-1 border rounded-md p-0.5">
              {(["24h", "7d", "30d"] as TimeRange[]).map((r) => (
                <Button
                  key={r}
                  variant={range === r ? "secondary" : "ghost"}
                  size="sm"
                  onClick={() => setRange(r)}
                  className="h-7"
                >
                  {r}
                </Button>
              ))}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => loadData()}
            >
              <RefreshCw className="h-4 w-4 mr-1" />
              {t("dashboard.refresh", "Refresh")}
            </Button>
          </div>
        </div>

      {/* Setup Banner */}
      <SetupBanner />

      {/* Stats Cards with Latency */}
      {loading ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
          {Array.from({ length: 5 }).map((_, i) => (
            <SkeletonCard key={i} />
          ))}
        </div>
      ) : (
        <>
          <div data-tour="dashboard-stats">
            <StatsCards range={range} />
          </div>
          {/* Latency Percentiles */}
          {latencyStatsData && (
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-8">
                  <div className="flex items-center gap-2">
                    <Clock className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm text-muted-foreground">Latency</span>
                  </div>
                  <div className="flex gap-6">
                    <div>
                      <span className="text-2xl font-bold">{formatMs(latencyStatsData.stats?.avgMs || latencyStatsData.avg || 0)}</span>
                      <span className="text-xs text-muted-foreground ml-1">P50</span>
                    </div>
                    <div>
                      <span className="text-lg font-semibold text-muted-foreground">{formatMs(latencyStatsData.stats?.maxMs || latencyStatsData.p95 || 0)}</span>
                      <span className="text-xs text-muted-foreground ml-1">P95</span>
                    </div>
                    <div>
                      <span className="text-lg font-semibold text-muted-foreground">{formatMs(latencyStatsData.stats?.minMs || latencyStatsData.p99 || 0)}</span>
                      <span className="text-xs text-muted-foreground ml-1">P99</span>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </>
      )}

      {/* Token Chart with Bucket Selector */}
      <Card data-tour="dashboard-chart">
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>{t("dashboard.tokenUsage", "Token Usage")}</CardTitle>
            <CardDescription>
              {t("dashboard.tokenUsageDesc", "API token consumption over time")}
            </CardDescription>
          </div>
          <div className="flex gap-1 border rounded-md p-0.5">
            {(["hour", "day"] as TimeseriesBucket[]).map((b) => (
              <Button
                key={b}
                variant={bucket === b ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setBucket(b)}
                className="h-7 capitalize"
              >
                {b === "hour" ? t("dashboard.hour", "Hour") : t("dashboard.day", "Day")}
              </Button>
            ))}
          </div>
        </CardHeader>
        <CardContent>
          {loading ? <SkeletonChart /> : <TokenChart range={range} bucket={bucket} />}
        </CardContent>
      </Card>

      {/* By-Model Stats Table */}
      {statsData && statsData.rows && statsData.rows.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.byModel", "Usage by Model")}</CardTitle>
            <CardDescription>
              {t("dashboard.byModelDesc", "Token usage breakdown by provider and model")}
            </CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("dashboard.provider", "Provider")}</TableHead>
                  <TableHead>{t("dashboard.model", "Model")}</TableHead>
                  <TableHead className="text-right">{t("dashboard.requests", "Requests")}</TableHead>
                  <TableHead className="text-right">{t("dashboard.errors", "Errors")}</TableHead>
                  <TableHead className="text-right">{t("dashboard.promptTokens", "Prompt")}</TableHead>
                  <TableHead className="text-right">{t("dashboard.completionTokens", "Completion")}</TableHead>
                  <TableHead className="text-right">{t("dashboard.totalTokens", "Total")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {statsData.rows.map((row: any, idx: number) => (
                  <TableRow key={idx}>
                    <TableCell><Badge variant="outline">{row.provider_id}</Badge></TableCell>
                    <TableCell><code className="text-sm">{row.upstream_model}</code></TableCell>
                    <TableCell className="text-right font-mono">{formatNumber(row.requests)}</TableCell>
                    <TableCell className="text-right font-mono text-destructive">{formatNumber(row.errors)}</TableCell>
                    <TableCell className="text-right font-mono">{formatTokens(row.prompt_tokens)}</TableCell>
                    <TableCell className="text-right font-mono">{formatTokens(row.completion_tokens)}</TableCell>
                    <TableCell className="text-right font-mono font-medium">{formatTokens(row.total_tokens)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}

      {/* Error Stats */}
      {errorStatsData && errorStatsData.error_codes && errorStatsData.error_codes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.topErrors", "Top Errors")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {errorStatsData.error_codes.slice(0, 12).map((err: any, idx: number) => (
                <Badge key={idx} variant="destructive" className="font-mono">
                  {err.code} × {formatNumber(err.count)}
                </Badge>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Two Column Layout */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Provider Health */}
        <Card data-tour="dashboard-health">
          <CardHeader>
            <CardTitle>{t("dashboard.providerHealth", "Provider Health")}</CardTitle>
            <CardDescription>
              {t("dashboard.providerHealthDesc", "Status of your configured providers")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading ? <Skeleton className="h-32 w-full" /> : <ProviderHealthCard />}
          </CardContent>
        </Card>

        {/* Recent Logs */}
        <Card data-tour="dashboard-logs">
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle>{t("dashboard.recentLogs", "Recent Logs")}</CardTitle>
            <Button variant="ghost" size="sm" asChild>
              <a href="/logs">
                {t("dashboard.viewAll", "View All")}
                <ExternalLink className="h-3 w-3 ml-1" />
              </a>
            </Button>
          </CardHeader>
          <CardContent>
            {loading ? <SkeletonTable rows={5} /> : <RecentLogsTable />}
          </CardContent>
        </Card>
      </div>
    </div>
    </PageTour>
  )
}