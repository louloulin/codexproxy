import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Card, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { ExternalLink } from "lucide-react"

const SETUP_BANNER_KEY = "rcodex.setup-banner-dismissed"

export function SetupBanner() {
  const { t } = useTranslation()
  const [dismissed, setDismissed] = useState(() => {
    if (typeof window === "undefined") return false
    return localStorage.getItem(SETUP_BANNER_KEY) === "1"
  })

  if (dismissed) {
    return null
  }

  function dismiss() {
    setDismissed(true)
    try {
      localStorage.setItem(SETUP_BANNER_KEY, "1")
    } catch {
      // ignore
    }
  }

  return (
    <Card className="bg-gradient-to-r from-primary/10 via-primary/5 to-transparent border-primary/20">
      <CardContent className="flex items-center justify-between py-4">
        <div className="flex items-center gap-4">
          <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/20">
            <ExternalLink className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h3 className="font-semibold">{t("setupBanner.title", "Welcome to rcodex!")}</h3>
            <p className="text-sm text-muted-foreground">
              {t(
                "setupBanner.desc",
                "Get started by configuring your Codex settings and adding your API providers."
              )}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="default" size="sm" asChild>
            <a href="/codex">
              {t("setupBanner.configure", "Configure Codex")}
            </a>
          </Button>
          <Button variant="ghost" size="sm" onClick={dismiss}>
            {t("setupBanner.dismiss", "Dismiss")}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}