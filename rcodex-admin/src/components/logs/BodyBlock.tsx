import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { ChevronDown, ChevronRight, FileText } from "lucide-react"
import { useState } from "react"

/**
 * 格式化 JSON 或返回原始文本
 */
function formatBody(text: string | null): string {
  if (!text) return ""
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
}

interface BodyBlockProps {
  title: string
  body: string | null
  defaultExpanded?: boolean
  maxHeight?: number
}

export function BodyBlock({
  title,
  body,
  defaultExpanded = false,
  maxHeight = 360,
}: BodyBlockProps) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(defaultExpanded)

  if (!body) {
    return (
      <Card className="border-dashed">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
            <FileText className="h-4 w-4" />
            {title}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground italic">
            {t("logs.expand.notCaptured", "未捕获请求/响应内容")}
          </p>
        </CardContent>
      </Card>
    )
  }

  const formattedBody = formatBody(body)
  const charCount = body.length

  return (
    <Card>
      <CardHeader className="pb-2">
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-2 w-full text-left hover:bg-muted/50 -ml-2 px-2 py-1 rounded-md transition-colors"
        >
          {expanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <CardTitle className="text-sm font-medium flex-1">
            {title}
          </CardTitle>
          <Badge variant="secondary" className="text-xs">
            {charCount.toLocaleString()} {t("logs.expand.chars", "字符")}
          </Badge>
        </button>
      </CardHeader>

      {expanded && (
        <CardContent className="pt-0">
          <div className={cn("rounded-md border bg-muted/30 overflow-auto")} style={{ maxHeight }}>
            <pre className="p-3 text-xs leading-relaxed overflow-auto whitespace-pre-wrap break-words font-mono m-0">
              {formattedBody}
            </pre>
          </div>
        </CardContent>
      )}
    </Card>
  )
}

/**
 * RequestBody - 请求体显示组件
 */
export function RequestBody({ body, defaultExpanded = false }: { body: string | null; defaultExpanded?: boolean }) {
  return <BodyBlock title={arguments[2] || "Request Body"} body={body} defaultExpanded={defaultExpanded} />
}

/**
 * ResponseBody - 响应体显示组件
 */
export function ResponseBody({ body, defaultExpanded = false }: { body: string | null; defaultExpanded?: boolean }) {
  return <BodyBlock title={arguments[2] || "Response Body"} body={body} defaultExpanded={defaultExpanded} />
}
