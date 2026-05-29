import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Globe } from "lucide-react"

export function LanguageSwitcher() {
  const { i18n } = useTranslation()

  const toggleLanguage = () => {
    const newLang = i18n.language === "en" ? "zh" : "en"
    i18n.changeLanguage(newLang)
    localStorage.setItem("i18nextLng", newLang)
  }

  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={toggleLanguage}
      title={i18n.language === "en" ? "Switch to Chinese" : "切换到 English"}
    >
      <Globe className="h-4 w-4 mr-1" />
      {i18n.language === "en" ? "中文" : "EN"}
    </Button>
  )
}
