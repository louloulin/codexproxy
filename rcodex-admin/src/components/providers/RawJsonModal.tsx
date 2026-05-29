import { useState, useMemo } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Copy, Clipboard, X, Check } from "lucide-react"

interface RawJsonModalProps {
  value: string
  onChange: (value: string) => void
  onCancel: () => void
  onSave: () => void
  isSaving?: boolean
}

export function RawJsonModal({ value, onChange, onCancel, onSave, isSaving }: RawJsonModalProps) {
  const [copied, setCopied] = useState(false)

  // Validate JSON
  const parsed = useMemo((): { ok: true } | { ok: false; error: string } => {
    try {
      JSON.parse(value)
      return { ok: true }
    } catch (e) {
      return { ok: false, error: (e as Error).message }
    }
  }, [value])

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (e) {
      console.error("Failed to copy:", e)
    }
  }

  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText()
      if (text) {
        onChange(text)
      }
    } catch (e) {
      console.error("Failed to paste:", e)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-[800px] max-h-[90vh] overflow-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Raw JSON Editor</CardTitle>
            <Button variant="ghost" size="icon" onClick={onCancel}>
              <X className="h-4 w-4" />
            </Button>
          </div>
          <p className="text-sm text-muted-foreground mt-1">
            Edit providers.json directly as JSON. Validate before saving.
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Actions */}
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={handleCopy}>
              {copied ? (
                <>
                  <Check className="h-4 w-4 mr-1" />
                  Copied!
                </>
              ) : (
                <>
                  <Copy className="h-4 w-4 mr-1" />
                  Copy
                </>
              )}
            </Button>
            <Button size="sm" variant="outline" onClick={handlePaste}>
              <Clipboard className="h-4 w-4 mr-1" />
              Paste from Clipboard
            </Button>
          </div>

          {/* JSON Editor */}
          <Input
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className={`font-mono text-xs min-h-[300px] ${
              !parsed.ok ? "border-destructive focus:border-destructive" : ""
            }`}
            style={{ minHeight: "300px" }}
          />

          {/* Validation Status */}
          {parsed.ok ? (
            <Alert variant="default" className="bg-emerald-50 border-emerald-200">
              <AlertDescription className="flex items-center gap-2 text-emerald-800">
                <Check className="h-4 w-4" />
                Valid JSON
              </AlertDescription>
            </Alert>
          ) : (
            <Alert variant="destructive">
              <AlertDescription>
                <span className="font-medium">Invalid JSON:</span> <code className="text-xs">{parsed.error}</code>
              </AlertDescription>
            </Alert>
          )}

          {/* Actions */}
          <div className="flex gap-2 justify-end pt-4">
            <Button variant="outline" onClick={onCancel}>
              Cancel
            </Button>
            <Button onClick={onSave} disabled={!parsed.ok || isSaving}>
              {isSaving ? "Saving..." : "Save"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
