import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Sparkles } from "lucide-react"

interface BilingualText {
  en: string
  zh: string
}

interface ReleaseHighlight {
  icon?: string
  kind?: "new" | "improved" | "fixed" | "doc"
  title: BilingualText
  description: BilingualText
  location?: BilingualText
  ctaLabel?: BilingualText
  ctaPath?: string
  ctaHref?: string
}

interface ReleaseNote {
  version: string
  date: string
  title: BilingualText
  summary?: BilingualText
  highlights: ReleaseHighlight[]
}

// 发布说明数据
const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: "0.1.0",
    date: "2026-05-30",
    title: {
      en: "rcodex Admin UI Launch",
      zh: "rcodex 管理面板正式发布",
    },
    summary: {
      en: "First release of rcodex Admin UI with complete Codex management, provider configuration, and statistics dashboard.",
      zh: "rcodex 管理面板首个版本发布，包含完整的 Codex 管理、Provider 配置和统计仪表盘。",
    },
    highlights: [
      {
        kind: "new",
        icon: "✨",
        title: {
          en: "Codex Configuration Management",
          zh: "Codex 配置管理",
        },
        description: {
          en: "Full support for configuring Codex CLI with multiple providers including MiniMax, OpenAI, and more.",
          zh: "完整支持配置 Codex CLI，支持 MiniMax、OpenAI 等多种 Provider。",
        },
        location: {
          en: "Sidebar → Codex",
          zh: "侧边栏 → Codex",
        },
      },
      {
        kind: "new",
        icon: "📊",
        title: {
          en: "Statistics Dashboard",
          zh: "统计仪表盘",
        },
        description: {
          en: "View token usage, request statistics, provider health, and more in real-time.",
          zh: "实时查看 Token 使用量、请求统计、Provider 健康状态等。",
        },
        location: {
          en: "Sidebar → Dashboard",
          zh: "侧边栏 → 仪表盘",
        },
      },
      {
        kind: "new",
        icon: "🔐",
        title: {
          en: "User Authentication",
          zh: "用户认证系统",
        },
        description: {
          en: "Secure login, registration, and user management with JWT tokens.",
          zh: "安全的登录、注册和用户管理系统，使用 JWT Token。",
        },
        location: {
          en: "Login Page",
          zh: "登录页面",
        },
      },
      {
        kind: "new",
        icon: "💾",
        title: {
          en: "Data Directory Management",
          zh: "数据目录管理",
        },
        description: {
          en: "Preview and migrate your data directory with real-time progress updates.",
          zh: "预览和迁移数据目录，实时显示进度。",
        },
        location: {
          en: "Settings → Data Directory",
          zh: "设置 → 数据目录",
        },
      },
      {
        kind: "fixed",
        icon: "🐛",
        title: {
          en: "MiniMax Provider Support",
          zh: "MiniMax Provider 支持",
        },
        description: {
          en: "Fixed MiniMax-M2.7 model support including streaming and reasoning modes.",
          zh: "修复 MiniMax-M2.7 模型支持，包括流式输出和推理模式。",
        },
      },
    ],
  },
]

const KIND_ICONS: Record<string, string> = {
  new: "✨",
  improved: "📈",
  fixed: "🔧",
  doc: "📝",
}

const KIND_COLORS: Record<string, string> = {
  new: "bg-blue-100 text-blue-800 border-blue-200",
  improved: "bg-purple-100 text-purple-800 border-purple-200",
  fixed: "bg-green-100 text-green-800 border-green-200",
  doc: "bg-gray-100 text-gray-800 border-gray-200",
}

const STORAGE_KEY = "rcodex:lastSeenVersion"

// 获取当前语言
function getCurrentLang(): string {
  return localStorage.getItem("i18nextLng") || "zh"
}

// 选择双语文本
function pick(text: BilingualText | undefined, lang: string): string {
  if (!text) return ""
  return lang.startsWith("zh") ? text.zh : text.en
}

// 比较版本号
function compareVersion(a: string, b: string): number {
  const parse = (v: string): number[] =>
    v.replace(/^v/, "").split(".").map((n) => {
      const m = /^(\d+)/.exec(n)
      return m ? parseInt(m[1], 10) : 0
    })
  const aa = parse(a)
  const bb = parse(b)
  const len = Math.max(aa.length, bb.length)
  for (let i = 0; i < len; i++) {
    const ai = aa[i] ?? 0
    const bi = bb[i] ?? 0
    if (ai !== bi) return ai - bi
  }
  return 0
}

// 获取未查看的发布说明
function getUnseenReleases(lastSeen: string | null, current: string): ReleaseNote[] {
  const baseline = lastSeen ?? "0.0.0"
  return RELEASE_NOTES.filter(
    (n) => compareVersion(n.version, baseline) > 0 && compareVersion(n.version, current) <= 0
  )
}

interface WhatsNewModalProps {
  currentVersion?: string
}

export function WhatsNewModal({ currentVersion = "0.1.0" }: WhatsNewModalProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [entries, setEntries] = useState<ReleaseNote[]>([])

  useEffect(() => {
    const lastSeen = localStorage.getItem(STORAGE_KEY)
    const list = getUnseenReleases(lastSeen, currentVersion)
    if (list.length > 0) {
      setEntries(list)
      setOpen(true)
    }
  }, [currentVersion])

  function handleClose(markSeen: boolean): void {
    if (markSeen && currentVersion) {
      // 标记当前版本为已查看
      localStorage.setItem(STORAGE_KEY, currentVersion)
    }
    setOpen(false)
  }

  if (entries.length === 0) return null

  const lang = getCurrentLang()
  const isMultiple = entries.length > 1

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && handleClose(false)}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-xl">
            <Sparkles className="h-5 w-5 text-amber-500" />
            🎉 {t("whatsNew.title", "What's New / 新功能介绍")}
          </DialogTitle>
          <DialogDescription>
            {isMultiple
              ? t("whatsNew.subtitleMulti", { count: entries.length })
              : t("whatsNew.subtitle", "查看最新更新内容")}
          </DialogDescription>
        </DialogHeader>

        <div className="mt-4 space-y-6">
          {entries.map((note, idx) => (
            <div key={note.version} className={idx > 0 ? "pt-6 border-t" : ""}>
              {/* 版本标题 */}
              <div className="flex items-baseline gap-3 mb-3">
                <h3 className="text-lg font-semibold">{pick(note.title, lang)}</h3>
                <span className="text-sm text-muted-foreground">
                  v{note.version} · {note.date}
                </span>
              </div>

              {/* 摘要 */}
              {note.summary && (
                <p className="text-sm text-muted-foreground mb-4">
                  {pick(note.summary, lang)}
                </p>
              )}

              {/* 亮点列表 */}
              <div className="space-y-3">
                {note.highlights.map((highlight, hIdx) => {
                  const kind = highlight.kind ?? "new"
                  const icon = highlight.icon ?? KIND_ICONS[kind] ?? "•"
                  const colorClass = KIND_COLORS[kind] ?? KIND_COLORS.doc

                  return (
                    <div
                      key={hIdx}
                      className="flex gap-3 p-3 rounded-lg border bg-card"
                    >
                      <div className="text-xl leading-6">{icon}</div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <Badge variant="outline" className={`text-xs ${colorClass}`}>
                            {t(`whatsNew.kind.${kind}`, kind === "new" ? "新功能" : kind === "improved" ? "改进" : kind === "fixed" ? "修复" : "文档")}
                          </Badge>
                          <strong className="text-sm">{pick(highlight.title, lang)}</strong>
                        </div>
                        <p className="text-sm text-muted-foreground mt-1 leading-relaxed">
                          {pick(highlight.description, lang)}
                        </p>
                        {highlight.location && (
                          <p className="text-xs text-muted-foreground mt-1">
                            <span className="font-medium">{t("whatsNew.location", "位置")}:</span>{" "}
                            {pick(highlight.location, lang)}
                          </p>
                        )}
                        {highlight.ctaLabel && (highlight.ctaPath || highlight.ctaHref) && (
                          <Button
                            variant="link"
                            size="sm"
                            className="mt-1 h-auto p-0 text-sm"
                            onClick={() => {
                              if (highlight.ctaHref) {
                                window.open(highlight.ctaHref, "_blank", "noopener,noreferrer")
                              } else if (highlight.ctaPath) {
                                window.location.href = highlight.ctaPath
                              }
                              handleClose(true)
                            }}
                          >
                            {pick(highlight.ctaLabel, lang)} →
                          </Button>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          ))}
        </div>

        <DialogFooter className="mt-6">
          <div className="flex justify-between w-full">
            <Button variant="ghost" size="sm" onClick={() => handleClose(true)}>
              {t("whatsNew.dontShowAgain", "不再显示")}
            </Button>
            <Button size="sm" onClick={() => handleClose(true)}>
              {t("common.close", "关闭")}
            </Button>
          </div>
        </DialogFooter>

        <p className="text-xs text-muted-foreground text-center mt-4">
          {t("whatsNew.footerHint", "版本信息会在首次启动后显示。可在设置中查看历史更新。")}
        </p>
      </DialogContent>
    </Dialog>
  )
}
