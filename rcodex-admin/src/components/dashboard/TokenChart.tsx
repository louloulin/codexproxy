import { useQuery } from "@tanstack/react-query"
import { useState, useMemo } from "react"
import { api } from "@/lib/api"
import { SkeletonChart } from "@/components/ui/skeleton"

type TimeRange = "24h" | "7d" | "30d"
type TimeseriesBucket = "hour" | "day"

interface TimeseriesPoint {
  ts: number
  prompt_tokens: number
  completion_tokens: number
  cached_tokens: number
}

interface TokenTimeseriesSeries {
  provider_id: string
  model: string
  points: TimeseriesPoint[]
  total?: number
}

interface TokenTimeseriesResponse {
  range: string
  series: Array<{
    provider_id: string
    model: string
    points: TimeseriesPoint[]
  }>
}

// Format large numbers
function formatTokens(n: number): string {
  if (n < 1000) return String(n)
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`
  return `${(n / 1_000_000).toFixed(2)}M`
}

// Calculate totals from series data
function calculateSeriesTotals(series: TokenTimeseriesResponse["series"]) {
  let prompt = 0, completion = 0, cached = 0
  for (const s of series) {
    for (const p of s.points) {
      prompt += p.prompt_tokens
      completion += p.completion_tokens
      cached += p.cached_tokens
    }
  }
  return { prompt, completion, cached }
}

// Calculate cache hit rate
function calculateCacheRate(series: TokenTimeseriesResponse["series"] | undefined | null): number | null {
  if (!series || series.length === 0) return null
  const totals = calculateSeriesTotals(series)
  if (totals.prompt === 0) return null
  return (totals.cached / totals.prompt) * 100
}

// Palette for chart series
const SERIES_COLORS = [
  "#6366f1", // indigo
  "#10b981", // emerald
  "#f59e0b", // amber
  "#ef4444", // red
  "#8b5cf6", // violet
  "#06b6d4", // cyan
  "#f97316", // orange
  "#ec4899", // pink
]

// Generate smooth path using Fritsch-Carlson monotone cubic interpolation
function generateSmoothPath(points: [number, number][]): string {
  const n = points.length
  if (n === 0) return ""
  if (n === 1) return `M${points[0][0]},${points[0][1]}`
  if (n === 2) return `M${points[0][0]},${points[0][1]} L${points[1][0]},${points[1][1]}`

  const dx: number[] = []
  const m: number[] = []
  for (let i = 0; i < n - 1; i++) {
    const d = points[i + 1][0] - points[i][0]
    dx.push(d)
    m.push(d === 0 ? 0 : (points[i + 1][1] - points[i][1]) / d)
  }

  const tan: number[] = new Array(n)
  tan[0] = m[0]
  tan[n - 1] = m[n - 2]
  for (let i = 1; i < n - 1; i++) {
    tan[i] = m[i - 1] * m[i] <= 0 ? 0 : (m[i - 1] + m[i]) / 2
  }

  // Monotonicity correction (Fritsch & Carlson 1980)
  for (let i = 0; i < n - 1; i++) {
    if (m[i] === 0) {
      tan[i] = 0
      tan[i + 1] = 0
    } else {
      const a = tan[i] / m[i]
      const b = tan[i + 1] / m[i]
      const h = a * a + b * b
      if (h > 9) {
        const t = 3 / Math.sqrt(h)
        tan[i] = t * a * m[i]
        tan[i + 1] = t * b * m[i]
      }
    }
  }

  let d = `M${points[0][0]},${points[0][1]}`
  for (let i = 0; i < n - 1; i++) {
    const cp1x = points[i][0] + dx[i] / 3
    const cp1y = points[i][1] + (tan[i] * dx[i]) / 3
    const cp2x = points[i + 1][0] - dx[i] / 3
    const cp2y = points[i + 1][1] - (tan[i + 1] * dx[i]) / 3
    d += ` C${cp1x},${cp1y} ${cp2x},${cp2y} ${points[i + 1][0]},${points[i + 1][1]}`
  }
  return d
}

interface MultiSeriesChartProps {
  series: TokenTimeseriesSeries[]
  width?: number
  height?: number
  onHover?: (index: number | null, x: number) => void
  hoverIndex?: number | null
}

function MultiSeriesChart({ series, width = 600, height = 200, onHover, hoverIndex }: MultiSeriesChartProps) {
  const padding = { top: 12, right: 12, bottom: 24, left: 48 }
  const chartWidth = width - padding.left - padding.right
  const chartHeight = height - padding.top - padding.bottom

  // Find max value across all series
  const maxValue = useMemo(() => {
    let max = 0
    for (const s of series) {
      for (const p of s.points) {
        const total = p.prompt_tokens + p.completion_tokens
        if (total > max) max = total
      }
    }
    // Nice ceiling
    if (max === 0) return 1
    const pow = Math.pow(10, Math.floor(Math.log10(max)))
    const frac = max / pow
    if (frac <= 1) return pow
    if (frac <= 2) return 2 * pow
    if (frac <= 5) return 5 * pow
    return 10 * pow
  }, [series])

  const pointCount = series[0]?.points.length || 1
  const scaleX = (i: number) => padding.left + (i / Math.max(pointCount - 1, 1)) * chartWidth
  const scaleY = (v: number) => padding.top + chartHeight - (v / maxValue) * chartHeight

  // Generate points for a series
  function getPoints(seriesIndex: number): [number, number][] {
    const s = series[seriesIndex]
    if (!s) return []
    return s.points.map((p, i) => [
      scaleX(i),
      scaleY(p.prompt_tokens + p.completion_tokens)
    ])
  }

  // Y-axis ticks
  const yTicks = [0, 0.25, 0.5, 0.75, 1].map(ratio => ({
    value: maxValue * ratio,
    y: scaleY(maxValue * ratio)
  }))

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="w-full"
      onMouseMove={(e) => {
        if (!onHover || series.length === 0) return
        const rect = e.currentTarget.getBoundingClientRect()
        const svgX = ((e.clientX - rect.left) / rect.width) * width
        const relativeX = svgX - padding.left
        if (pointCount <= 1) {
          onHover(0, svgX)
          return
        }
        const index = Math.round((relativeX / chartWidth) * (pointCount - 1))
        const clamped = Math.max(0, Math.min(pointCount - 1, index))
        onHover(clamped, svgX)
      }}
      onMouseLeave={() => onHover?.(null, 0)}
      style={{ cursor: "crosshair" }}
    >
      {/* Y-axis grid lines and labels */}
      {yTicks.map((tick, i) => (
        <g key={i}>
          <line
            x1={padding.left}
            x2={width - padding.right}
            y1={tick.y}
            y2={tick.y}
            stroke="currentColor"
            strokeOpacity="0.1"
            strokeDasharray="2 4"
          />
          <text
            x={padding.left - 8}
            y={tick.y + 4}
            textAnchor="end"
            className="fill-muted-foreground text-[10px]"
          >
            {formatTokens(tick.value)}
          </text>
        </g>
      ))}

      {/* Series lines and areas */}
      {series.map((s, seriesIndex) => {
        const color = SERIES_COLORS[seriesIndex % SERIES_COLORS.length]
        const points = getPoints(seriesIndex)
        if (points.length === 0) return null

        // Generate area path (fill to bottom)
        const baselineY = padding.top + chartHeight
        const areaPath = generateSmoothPath(points) +
          ` L${points[points.length - 1]?.[0] || 0},${baselineY}` +
          ` L${points[0]?.[0] || 0},${baselineY} Z`

        return (
          <g key={s.provider_id + s.model}>
            {/* Area fill */}
            <defs>
              <linearGradient id={`gradient-${seriesIndex}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={color} stopOpacity="0.2" />
                <stop offset="100%" stopColor={color} stopOpacity="0.02" />
              </linearGradient>
            </defs>
            <path d={areaPath} fill={`url(#gradient-${seriesIndex})`} />
            {/* Line */}
            <path
              d={generateSmoothPath(points)}
              fill="none"
              stroke={color}
              strokeWidth="2"
              strokeLinejoin="round"
              strokeLinecap="round"
            />
            {/* Hover indicator dot */}
            {hoverIndex !== null && hoverIndex !== undefined && points[hoverIndex] && (
              <circle
                cx={points[hoverIndex][0]}
                cy={points[hoverIndex][1]}
                r={5}
                fill={color}
                stroke="white"
                strokeWidth={2}
              />
            )}
          </g>
        )
      })}

      {/* Hover vertical line */}
      {hoverIndex !== null && hoverIndex !== undefined && (
        <line
          x1={scaleX(hoverIndex)}
          x2={scaleX(hoverIndex)}
          y1={padding.top}
          y2={padding.top + chartHeight}
          stroke="currentColor"
          strokeOpacity="0.3"
          strokeDasharray="3 3"
        />
      )}

      {/* X-axis labels */}
      {series[0]?.points.filter((_, i) => {
        return i % Math.ceil(pointCount / 6) === 0 || i === pointCount - 1
      }).map((p) => {
        const originalIndex = series[0].points.indexOf(p)
        return (
          <text
            key={originalIndex}
            x={scaleX(originalIndex)}
            y={height - 6}
            textAnchor="middle"
            className="fill-muted-foreground text-[10px]"
          >
            {new Date(p.ts).toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })}
          </text>
        )
      })}
    </svg>
  )
}

export function TokenChart({ range, bucket }: { range: TimeRange; bucket?: TimeseriesBucket }) {
  const actualBucket: TimeseriesBucket = bucket || (range === "24h" ? "hour" : "day")
  const [hoverIndex, setHoverIndex] = useState<number | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ["token-timeseries", range, actualBucket],
    queryFn: async () => {
      try {
        return await api.stats.tokenTimeseries(range, actualBucket)
      } catch {
        // Return null for unavailable API
        return null
      }
    },
    refetchInterval: 30000,
  })

  const cacheRate = calculateCacheRate(data?.series)

  // Calculate totals from series
  const totals = data?.series ? calculateSeriesTotals(data.series) : { prompt: 0, completion: 0, cached: 0 }

  // Calculate series totals for legend
  const seriesWithTotals = useMemo((): TokenTimeseriesSeries[] => {
    if (!data?.series) return []
    return data.series.map(s => ({
      provider_id: s.provider_id,
      model: s.model,
      points: s.points,
      total: s.points.reduce((sum, p) => sum + p.prompt_tokens + p.completion_tokens, 0)
    })).sort((a, b) => b.total - a.total)
  }, [data])

  // Calculate totals for hover tooltip
  const hoverTotals = useMemo(() => {
    if (hoverIndex === null || !data?.series) return null
    let prompt = 0, completion = 0, cached = 0
    for (const s of data.series) {
      const p = s.points[hoverIndex]
      if (p) {
        prompt += p.prompt_tokens
        completion += p.completion_tokens
        cached += p.cached_tokens
      }
    }
    return { prompt, completion, cached, timestamp: data.series[0]?.points[hoverIndex]?.ts }
  }, [data, hoverIndex])

  if (isLoading) {
    return <SkeletonChart />
  }

  return (
    <div className="space-y-4">
      {/* Summary */}
      <div className="flex items-baseline gap-4 flex-wrap">
        <div>
          <span className="text-2xl font-bold">{formatTokens(totals.prompt)}</span>
          <span className="text-muted-foreground ml-1">prompt</span>
        </div>
        <div>
          <span className="text-2xl font-bold text-emerald-600">{formatTokens(totals.completion)}</span>
          <span className="text-muted-foreground ml-1">completion</span>
        </div>
        {cacheRate !== null && (
          <div>
            <span className="text-lg font-semibold text-amber-600">{cacheRate.toFixed(1)}%</span>
            <span className="text-muted-foreground ml-1">cached</span>
          </div>
        )}
      </div>

      {/* Hover tooltip */}
      {hoverTotals && hoverTotals.timestamp && (
        <div className="bg-muted/50 rounded-md px-3 py-2 text-sm">
          <div className="font-medium mb-1">
            {new Date(hoverTotals.timestamp).toLocaleString()}
          </div>
          <div className="grid grid-cols-3 gap-4 text-xs">
            <div>
              <span className="text-muted-foreground">Prompt:</span>{" "}
              <span className="font-mono font-medium">{formatTokens(hoverTotals.prompt)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Completion:</span>{" "}
              <span className="font-mono font-medium">{formatTokens(hoverTotals.completion)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Cached:</span>{" "}
              <span className="font-mono font-medium text-amber-600">{formatTokens(hoverTotals.cached)}</span>
            </div>
          </div>
        </div>
      )}

      {/* Chart */}
      {seriesWithTotals.length > 0 ? (
        <MultiSeriesChart
          series={seriesWithTotals}
          onHover={setHoverIndex}
          hoverIndex={hoverIndex}
        />
      ) : (
        <div className="flex items-center justify-center h-48 text-muted-foreground">
          {data?.series?.length === 0 ? "No data available" : "Loading..."}
        </div>
      )}

      {/* Legend */}
      <div className="flex flex-wrap gap-4 pt-2 border-t">
        {seriesWithTotals.map((s, i) => (
          <div key={s.provider_id + s.model} className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-full"
              style={{ backgroundColor: SERIES_COLORS[i % SERIES_COLORS.length] }}
            />
            <span className="text-xs">
              <span className="text-muted-foreground">{s.provider_id}/</span>
              <code className="font-mono">{s.model}</code>
            </span>
            <span className="text-xs text-muted-foreground font-mono">
              {formatTokens(s.total || 0)}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}