import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import type { ProviderInfo } from "@/types/codex"

interface KeyStatusBannerProps {
  providers: ProviderInfo[]
}

/**
 * KeyStatusBanner - 显示API Key状态提示组件
 * 参考 mimo2codex/web/src/components/KeyStatusBanner.tsx 实现
 */
export function KeyStatusBanner({ providers }: KeyStatusBannerProps) {
  // 找出缺少 API Key 的 provider
  const missing = providers.filter((p) => !p.api_key_present)

  // 如果所有 provider 都有 key，显示成功提示
  if (missing.length === 0) {
    return (
      <Alert className="border-green-200 bg-green-50">
        <AlertTitle className="text-green-800">✓ 所有 Provider 已配置</AlertTitle>
        <AlertDescription className="text-green-700">
          所有 Provider 的 API Key 均已正确配置。
        </AlertDescription>
      </Alert>
    )
  }

  // 获取第一个缺失的 provider 的环境变量名
  const firstEnv = missing[0]?.api_key_env?.[0]

  return (
    <Alert variant="warning" className="border-amber-200 bg-amber-50">
      <AlertTitle className="text-amber-800">⚠️ API Key 缺失</AlertTitle>
      <AlertDescription className="text-amber-700">
        <div className="mt-2">
          <p className="font-medium">以下 Provider 缺少 API Key:</p>
          <ul className="mt-2 ml-4 list-disc space-y-1">
            {missing.map((p) => (
              <li key={p.id}>
                <span className="font-medium">{p.display_name}</span>:{" "}
                <code className="bg-amber-100 px-1 py-0.5 rounded text-xs">
                  {p.api_key_env?.join(" / ")}
                </code>
              </li>
            ))}
          </ul>

          <div className="mt-3 p-3 bg-amber-100 rounded-lg text-sm">
            <p className="font-medium text-amber-900">配置示例:</p>
            <div className="mt-1 space-y-1 font-mono text-xs text-amber-800">
              <p>
                <span className="text-amber-600"># macOS / Linux</span>
              </p>
              <p>export {firstEnv}=your-api-key-here</p>
              <p className="mt-2 text-amber-600"># Windows (PowerShell)</p>
              <p>$env:{firstEnv}="your-api-key-here"</p>
            </div>
          </div>
        </div>
      </AlertDescription>
    </Alert>
  )
}
