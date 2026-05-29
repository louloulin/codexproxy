import { useTranslation } from "react-i18next"
import { Bell, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { LanguageSwitcher } from "@/components/LanguageSwitcher"

interface HeaderProps {
  onRefresh?: () => void
  refreshing?: boolean
}

export function Header({ onRefresh, refreshing }: HeaderProps) {
  const { t } = useTranslation()

  return (
    <header className="sticky top-0 z-20 bg-background border-b border-border px-6 py-3 flex items-center justify-between">
      <div className="flex items-center gap-4">
        <h1 className="text-xl font-semibold">
          {t("header.title", { defaultValue: "Dashboard" })}
        </h1>
      </div>

      <div className="flex items-center gap-2">
        {onRefresh && (
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            disabled={refreshing}
          >
            <RefreshCw className={`h-4 w-4 mr-1 ${refreshing ? "animate-spin" : ""}`} />
            {refreshing ? t("header.refreshing") : t("header.refresh")}
          </Button>
        )}
        <Button variant="ghost" size="icon">
          <Bell className="h-4 w-4" />
        </Button>
        <LanguageSwitcher />
      </div>
    </header>
  )
}