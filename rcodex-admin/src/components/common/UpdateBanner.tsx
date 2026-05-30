import { useState, useEffect } from "react"
import { useQuery } from "@tanstack/react-query"
import { api } from "@/lib/api"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { RefreshCw, ExternalLink, X } from "lucide-react"

interface UpdateStatus {
  currentVersion: string
  latestVersion: string | null
  updateAvailable: boolean
  releaseNotes: string | null
  releaseUrl: string | null
  checkedAt: string | null
}

export function UpdateBanner() {
  const [dismissed, setDismissed] = useState(false)

  // Fetch update status
  const { data: status, refetch } = useQuery<UpdateStatus>({
    queryKey: ["update", "status"],
    queryFn: () => api.update.status(),
    staleTime: 1000 * 60 * 60, // 1 hour cache
    retry: 1,
  })

  // Auto-check on mount
  useEffect(() => {
    if (!status) {
      refetch()
    }
  }, [])

  // Don't show if dismissed or no update available
  if (dismissed || !status?.updateAvailable) {
    return null
  }

  return (
    <Alert className="mx-4 mt-4 bg-amber-50 border-amber-200">
      <AlertDescription className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <RefreshCw className="h-4 w-4 text-amber-600" />
          <div className="flex-1">
            <p className="font-medium text-amber-800">
              Update Available: v{status.latestVersion}
            </p>
            <p className="text-sm text-amber-700">
              You're running v{status.currentVersion}. A newer version is available.
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {status.releaseUrl && (
            <Button
              variant="outline"
              size="sm"
              className="border-amber-300 text-amber-800 hover:bg-amber-100"
              onClick={() => window.open(status.releaseUrl!, "_blank")}
            >
              <ExternalLink className="h-4 w-4 mr-1" />
              Release Notes
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-amber-600 hover:text-amber-800 hover:bg-amber-100"
            onClick={() => setDismissed(true)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </AlertDescription>
    </Alert>
  )
}
