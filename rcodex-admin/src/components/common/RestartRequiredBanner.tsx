import { useEffect, useState } from "react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { RefreshCw } from "lucide-react"

interface HealthResponse {
  ok: boolean
  dataDir: string
  version: string
  authMode: "off" | "on"
  maintenance?: boolean
  restartRequired?: boolean
  restartReason?: string | null
  restartTargetDir?: string | null
}

/**
 * RestartRequiredBanner - 显示重启提示组件
 *
 * 每30秒轮询 /health 接口。当服务器设置 restartRequired=true 时（例如数据目录迁移完成），
 * 显示一个警告横幅，提示用户需要重启服务。
 *
 * 参考 mimo2codex/web/src/components/RestartRequiredBanner.tsx 实现
 */
const POLL_INTERVAL_MS = 30_000

export function RestartRequiredBanner() {
  const [health, setHealth] = useState<HealthResponse | null>(null)

  useEffect(() => {
    let cancelled = false
    let timer: ReturnType<typeof setInterval> | null = null

    async function fetchHealth(): Promise<void> {
      try {
        const response = await fetch("/health")
        if (response.ok) {
          const data = await response.json()
          if (!cancelled) {
            setHealth(data)
          }
        }
      } catch {
        // 网络错误 - 保持最后的状态
      }
    }

    // 立即获取一次
    void fetchHealth()

    // 设置定时轮询
    timer = setInterval(fetchHealth, POLL_INTERVAL_MS)

    return () => {
      cancelled = true
      if (timer !== null) {
        clearInterval(timer)
      }
    }
  }, [])

  // 如果不需要重启，返回 null
  if (!health?.restartRequired) {
    return null
  }

  const targetDir = health.restartTargetDir ?? health.dataDir

  return (
    <Alert variant="warning" className="border-amber-200 bg-amber-50">
      <div className="flex items-start gap-3">
        <RefreshCw className="h-5 w-5 text-amber-600 mt-0.5" />
        <div className="flex-1">
          <AlertTitle className="text-amber-800 flex items-center gap-2">
            <RefreshCw className="h-4 w-4" />
            需要重启服务
          </AlertTitle>
          <AlertDescription className="text-amber-700 mt-2">
            <p>配置已更新，需要重启服务才能生效。</p>
            {health.restartReason && (
              <p className="mt-1 text-sm">
                <span className="font-medium">原因:</span> {health.restartReason}
              </p>
            )}
            {targetDir && (
              <p className="mt-1 text-sm">
                <span className="font-medium">数据目录:</span> {targetDir}
              </p>
            )}
            <div className="mt-3 p-3 bg-amber-100 rounded-lg">
              <p className="font-medium text-amber-900 text-sm">重启命令:</p>
              <code className="mt-1 block text-xs text-amber-800 font-mono">
                # 停止当前服务，然后重新启动
              </code>
            </div>
          </AlertDescription>
        </div>
      </div>
    </Alert>
  )
}
