import { useQuery } from "@tanstack/react-query"
import { api } from "@/lib/api"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Loader2, Copy, Check } from "lucide-react"
import { useState } from "react"

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error("Failed to copy:", err)
    }
  }

  return (
    <Button size="sm" variant="ghost" onClick={handleCopy} title="Copy to clipboard">
      {copied ? <Check className="h-4 w-4 text-green-500" /> : <Copy className="h-4 w-4" />}
    </Button>
  )
}

function CodeBlock({ code, language = "bash" }: { code: string; language?: string }) {
  return (
    <div className="relative">
      <div className="flex items-center justify-between bg-muted px-3 py-2 rounded-t-lg border">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {language}
        </span>
        <CopyButton text={code} />
      </div>
      <pre className="bg-muted/50 p-4 rounded-b-lg border border-t-0 overflow-x-auto">
        <code className="text-sm font-mono whitespace-pre">{code}</code>
      </pre>
    </div>
  )
}

export function SetupSnippets() {
  const { data: snippets, isLoading, error } = useQuery({
    queryKey: ["setup-snippets"],
    queryFn: () => api.codex.setupSnippets(),
  })

  if (isLoading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin" />
        </CardContent>
      </Card>
    )
  }

  if (error || !snippets?.ok || !snippets.data) {
    return (
      <Card>
        <CardContent className="py-4">
          <p className="text-destructive text-sm">Failed to load setup instructions</p>
        </CardContent>
      </Card>
    )
  }

  const { bundle, providers } = snippets.data
  const isMac = typeof navigator !== "undefined" && /mac/i.test(navigator.platform || "")
  const isWindows = typeof navigator !== "undefined" && /win/i.test(navigator.platform || "")

  return (
    <Card>
      <CardHeader>
        <CardTitle>Setup Instructions</CardTitle>
        <CardDescription>How to configure API keys for Codex</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Provider Selection */}
        <div>
          <h4 className="text-sm font-medium mb-2">Available Providers</h4>
          <div className="flex flex-wrap gap-2">
            {providers.map((p) => (
              <div
                key={p.id}
                className="px-3 py-1 bg-secondary rounded-full text-sm"
              >
                <span className="font-mono">{p.shortcut}</span>
                <span className="text-muted-foreground ml-1">{p.display_name}</span>
              </div>
            ))}
          </div>
        </div>

        <Tabs defaultValue={isMac ? "mac" : isWindows ? "windows" : "linux"} className="w-full">
          <TabsList>
            <TabsTrigger value="mac">macOS</TabsTrigger>
            <TabsTrigger value="linux">Linux</TabsTrigger>
            <TabsTrigger value="windows">Windows</TabsTrigger>
          </TabsList>

          <TabsContent value="mac" className="space-y-4">
            <div>
              <h4 className="text-sm font-medium mb-2">1. Set Environment Variable</h4>
              <CodeBlock
                language="bash"
                code={`# Add to your shell profile (~/.zshrc or ~/.bashrc)
export ${bundle.target.provider_key}="your-api-key-here"

# Apply changes
source ~/.zshrc  # or source ~/.bashrc`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">2. auth.json</h4>
              <CodeBlock
                language="json"
                code={`# Location: ~/.codex/auth.json
${bundle.auth_json}`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">3. config.toml</h4>
              <CodeBlock
                language="toml"
                code={`# Location: ~/.codex/config.toml
${bundle.config_toml}`}
              />
            </div>
          </TabsContent>

          <TabsContent value="linux" className="space-y-4">
            <div>
              <h4 className="text-sm font-medium mb-2">1. Set Environment Variable</h4>
              <CodeBlock
                language="bash"
                code={`# Add to your shell profile (~/.bashrc or ~/.profile)
export ${bundle.target.provider_key}="your-api-key-here"

# Apply changes
source ~/.bashrc`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">2. auth.json</h4>
              <CodeBlock
                language="json"
                code={`# Location: ~/.codex/auth.json
${bundle.auth_json}`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">3. config.toml</h4>
              <CodeBlock
                language="toml"
                code={`# Location: ~/.codex/config.toml
${bundle.config_toml}`}
              />
            </div>
          </TabsContent>

          <TabsContent value="windows" className="space-y-4">
            <div>
              <h4 className="text-sm font-medium mb-2">1. Set Environment Variable</h4>
              <CodeBlock
                language="powershell"
                code={`# PowerShell
$env:${bundle.target.provider_key} = "your-api-key-here"

# Or via System Properties
# Search "Environment Variables" in Start Menu`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">2. auth.json</h4>
              <CodeBlock
                language="json"
                code={`# Location: %USERPROFILE%\\.codex\\auth.json
${bundle.auth_json}`}
              />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-2">3. config.toml</h4>
              <CodeBlock
                language="toml"
                code={`# Location: %USERPROFILE%\\.codex\\config.toml
${bundle.config_toml}`}
              />
            </div>
          </TabsContent>
        </Tabs>

        {/* Model Info */}
        <div className="p-4 bg-muted rounded-lg">
          <h4 className="text-sm font-medium mb-2">Model Information</h4>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <span className="text-muted-foreground">Provider:</span>{" "}
              <span className="font-mono">{bundle.target.provider_label}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Model:</span>{" "}
              <span className="font-mono">{bundle.target.model_id}</span>
            </div>
            {bundle.target.context_window && (
              <div>
                <span className="text-muted-foreground">Context:</span>{" "}
                {bundle.target.context_window.toLocaleString()} tokens
              </div>
            )}
            {bundle.target.max_output_tokens && (
              <div>
                <span className="text-muted-foreground">Max Output:</span>{" "}
                {bundle.target.max_output_tokens.toLocaleString()} tokens
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
