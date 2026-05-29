import { useState, useEffect, useRef, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { X, ChevronLeft, ChevronRight, HelpCircle, MessageSquare } from "lucide-react"

export interface TourStep {
  target?: string  // CSS selector for the element to highlight
  title: string
  description: string
  placement?: "top" | "bottom" | "left" | "right" | "center"
}

interface PageTourProps {
  pageKey: string
  steps: TourStep[]
  children: React.ReactNode
}

const STORAGE_PREFIX = "rcodex-admin-tour-"

export function PageTour({ pageKey, steps, children }: PageTourProps) {
  const { t } = useTranslation()
  const [isOpen, setIsOpen] = useState(false)
  const [currentStep, setCurrentStep] = useState(0)
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null)
  const [initialized, setInitialized] = useState(false)
  const contentRef = useRef<HTMLDivElement>(null)

  const storageKey = `${STORAGE_PREFIX}${pageKey}-done`

  // Check if tour was completed
  useEffect(() => {
    try {
      const completed = localStorage.getItem(storageKey) === "true"
      if (!completed) {
        // Auto-open tour after a short delay to let page render
        const timer = setTimeout(() => setIsOpen(true), 500)
        return () => clearTimeout(timer)
      }
    } catch {
      // localStorage not available
    }
    setInitialized(true)
  }, [storageKey])

  // Update target element position
  useEffect(() => {
    if (!isOpen || !initialized) return

    const step = steps[currentStep]
    if (!step?.target) {
      setTargetRect(null)
      return
    }

    const updatePosition = () => {
      const element = document.querySelector(step.target!)
      if (element) {
        setTargetRect(element.getBoundingClientRect())
      } else {
        setTargetRect(null)
      }
    }

    updatePosition()
    window.addEventListener("resize", updatePosition)
    return () => window.removeEventListener("resize", updatePosition)
  }, [isOpen, currentStep, steps, initialized])

  const handleClose = useCallback(() => {
    setIsOpen(false)
    setCurrentStep(0)
    setTargetRect(null)
  }, [])

  const handleNext = useCallback(() => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1)
    } else {
      handleFinish()
    }
  }, [currentStep, steps.length])

  const handlePrev = useCallback(() => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1)
    }
  }, [currentStep])

  const handleFinish = useCallback(() => {
    try {
      localStorage.setItem(storageKey, "true")
    } catch {
      // localStorage not available
    }
    handleClose()
  }, [storageKey, handleClose])

  const handleSkip = useCallback(() => {
    try {
      localStorage.setItem(storageKey, "true")
    } catch {
      // localStorage not available
    }
    handleClose()
  }, [storageKey, handleClose])

  const currentTourStep = steps[currentStep]

  // Calculate tooltip position
  const getTooltipPosition = () => {
    if (!targetRect) {
      // Center position for intro/outro steps
      return {
        bottom: 24,
        right: 24,
        left: "auto",
        top: "auto",
        transform: "none",
      }
    }

    const tooltipWidth = 384  // w-96 = 24rem = 384px
    const tooltipHeight = 200  // Approximate
    const padding = 16
    const placement = currentTourStep?.placement || "bottom"

    let position: Record<string, string | number> = {}

    switch (placement) {
      case "top":
        position = {
          left: targetRect.left + targetRect.width / 2 - tooltipWidth / 2,
          top: targetRect.top - tooltipHeight - padding,
        }
        break
      case "bottom":
        position = {
          left: targetRect.left + targetRect.width / 2 - tooltipWidth / 2,
          top: targetRect.bottom + padding,
        }
        break
      case "left":
        position = {
          right: window.innerWidth - targetRect.left + padding,
          top: targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
        }
        break
      case "right":
        position = {
          left: targetRect.right + padding,
          top: targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
        }
        break
      default:
        position = {
          bottom: 24,
          right: 24,
        }
    }

    return position
  }

  if (!initialized) return <>{children}</>

  return (
    <>
      {children}

      {/* Floating Help Button */}
      <Button
        variant="outline"
        size="icon"
        className="fixed bottom-6 right-6 z-40 rounded-full shadow-lg hover:shadow-xl transition-shadow"
        onClick={() => setIsOpen(true)}
        title={t("app.help", "Help")}
      >
        <HelpCircle className="h-5 w-5" />
      </Button>

      {/* Tour Overlay */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div className="fixed inset-0 z-40" onClick={handleClose} />

          {/* Highlight Frame */}
          {targetRect && (
            <div
              className="fixed z-40 border-2 border-primary rounded-lg pointer-events-none"
              style={{
                left: targetRect.left - 4,
                top: targetRect.top - 4,
                width: targetRect.width + 8,
                height: targetRect.height + 8,
              }}
            />
          )}

          {/* Tour Card */}
          <Card
            ref={contentRef}
            className="fixed z-50 w-96 shadow-2xl animate-in slide-in-from-bottom-4 fade-in duration-300"
            style={{
              ...getTooltipPosition(),
              maxWidth: "calc(100vw - 32px)",
            }}
          >
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <CardTitle className="text-lg flex items-center gap-2">
                  <MessageSquare className="h-5 w-5 text-primary" />
                  {currentTourStep?.title}
                </CardTitle>
                <Button variant="ghost" size="icon" className="h-8 w-8" onClick={handleClose}>
                  <X className="h-4 w-4" />
                </Button>
              </div>
              <Badge variant="outline" className="w-fit">
                {currentStep + 1} / {steps.length}
              </Badge>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">{currentTourStep?.description}</p>

              {/* Progress Dots */}
              <div className="flex justify-center gap-1.5">
                {steps.map((_, index) => (
                  <div
                    key={index}
                    className={`h-2 w-2 rounded-full transition-colors ${
                      index === currentStep ? "bg-primary" : index < currentStep ? "bg-primary/50" : "bg-muted"
                    }`}
                  />
                ))}
              </div>

              {/* Navigation */}
              <div className="flex justify-between items-center pt-2">
                <Button variant="ghost" size="sm" onClick={handleSkip}>
                  Skip Tour
                </Button>
                <div className="flex gap-2">
                  {currentStep > 0 && (
                    <Button variant="outline" size="sm" onClick={handlePrev}>
                      <ChevronLeft className="h-4 w-4 mr-1" />
                      Back
                    </Button>
                  )}
                  <Button size="sm" onClick={handleNext}>
                    {currentStep < steps.length - 1 ? (
                      <>
                        Next
                        <ChevronRight className="h-4 w-4 ml-1" />
                      </>
                    ) : (
                      "Finish"
                    )}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </>
  )
}

// Hook to check if tour has been completed
export function useTourCompleted(pageKey: string) {
  const [completed, setCompleted] = useState(false)

  useEffect(() => {
    try {
      const value = localStorage.getItem(`${STORAGE_PREFIX}${pageKey}-done`)
      setCompleted(value === "true")
    } catch {
      setCompleted(false)
    }
  }, [pageKey])

  const markCompleted = useCallback(() => {
    try {
      localStorage.setItem(`${STORAGE_PREFIX}${pageKey}-done`, "true")
      setCompleted(true)
    } catch {
      // localStorage not available
    }
  }, [pageKey])

  const resetTour = useCallback(() => {
    try {
      localStorage.removeItem(`${STORAGE_PREFIX}${pageKey}-done`)
      setCompleted(false)
    } catch {
      // localStorage not available
    }
  }, [pageKey])

  return { completed, markCompleted, resetTour }
}
