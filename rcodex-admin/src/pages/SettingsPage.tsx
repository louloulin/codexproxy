import { useState } from "react"
import { useQuery, useMutation } from "@tanstack/react-query"
import { api } from "@/lib/api"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { FolderOpen, RefreshCw, HardDrive, AlertTriangle, CheckCircle } from "lucide-react"

export function SettingsPage() {
  const [targetDir, setTargetDir] = useState("")
  const [error, setError] = useState<string | null>(null)

  // Fetch data directory info
  const { data: dirInfo, isLoading: isLoadingInfo } = useQuery({
    queryKey: ["dataDir", "info"],
    queryFn: () => api.dataDir.info(),
  })

  // Preview migration mutation
  const previewMutation = useMutation({
    mutationFn: async (target: string) => {
      return api.dataDir.preview(target)
    },
    onSuccess: (data) => {
      if (!data.ok) {
        setError("Preview failed")
      }
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Preview failed")
    },
  })

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B"
    const k = 1024
    const sizes = ["B", "KB", "MB", "GB", "TB"]
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    const result = parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i]
    return result
  }

  function handlePreview() {
    if (!targetDir.trim()) {
      setError("Target directory is required")
      return
    }
    setError(null)
    previewMutation.mutate(targetDir.trim())
  }

  const preview = previewMutation.data

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Settings</h1>
        <p className="text-muted-foreground">Configure rcodex settings</p>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Data Directory Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <FolderOpen className="h-5 w-5" />
            <CardTitle>Data Directory</CardTitle>
          </div>
          <CardDescription>Manage where rcodex stores its data</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoadingInfo ? (
            <div className="flex items-center justify-center py-4">
              <RefreshCw className="h-6 w-6 animate-spin" />
            </div>
          ) : dirInfo ? (
            <>
              {/* Current Info */}
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                  <Label className="text-muted-foreground">Current Location</Label>
                  <p className="font-mono text-sm bg-muted p-2 rounded">
                    {dirInfo.current}
                  </p>
                </div>
                <div className="space-y-1">
                  <Label className="text-muted-foreground">Default Location</Label>
                  <p className="font-mono text-sm bg-muted p-2 rounded">
                    {dirInfo.defaultDir}
                  </p>
                </div>
              </div>

              {/* Migration Section */}
              <div className="border-t pt-4 mt-4">
                <div className="flex items-center gap-2 mb-4">
                  <HardDrive className="h-5 w-5 text-amber-500" />
                  <h3 className="font-semibold">Data Migration</h3>
                </div>

                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="targetDir">Target Directory</Label>
                    <div className="flex gap-2">
                      <Input
                        id="targetDir"
                        value={targetDir}
                        onChange={(e) => setTargetDir(e.target.value)}
                        placeholder="/path/to/new/data/directory"
                        className="flex-1"
                      />
                      <Button
                        variant="outline"
                        onClick={handlePreview}
                        disabled={previewMutation.isPending}
                      >
                        {previewMutation.isPending ? (
                          <RefreshCw className="h-4 w-4 animate-spin" />
                        ) : (
                          "Preview"
                        )}
                      </Button>
                    </div>
                  </div>

                  {/* Preview Results */}
                  {preview && (
                    <div className="bg-muted p-4 rounded-lg space-y-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span className="font-medium">Preview Results</span>
                      </div>
                      <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <span className="text-muted-foreground">Current:</span>
                          <p className="font-mono">{preview.currentDir}</p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Target:</span>
                          <p className="font-mono">{preview.targetDir}</p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Estimated Size:</span>
                          <p>{formatBytes(preview.estimatedBytes)}</p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Target Exists:</span>
                          <p>{preview.exists ? "Yes" : "No"}</p>
                        </div>
                      </div>

                      <Alert>
                        <AlertTriangle className="h-4 w-4" />
                        <AlertDescription>
                          Data migration requires server restart. The actual migration
                          copies files and updates configuration.
                        </AlertDescription>
                      </Alert>

                      <div className="flex gap-2">
                        <Button disabled>
                          Migrate & Restart (Coming Soon)
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </>
          ) : (
            <p className="text-muted-foreground">Failed to load data directory info</p>
          )}
        </CardContent>
      </Card>

      {/* About Card */}
      <Card>
        <CardHeader>
          <CardTitle>About rcodex</CardTitle>
          <CardDescription>OpenAI Proxy Server</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            rcodex is an OpenAI-compatible proxy server that routes requests to various
            AI providers including MiniMax, OpenAI, Anthropic, and more.
          </p>
          <div className="mt-4 text-sm">
            <p><span className="text-muted-foreground">Version:</span> 0.1.0</p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
