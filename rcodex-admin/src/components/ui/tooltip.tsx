import * as React from "react"
import { cn } from "@/lib/utils"

interface TooltipProps {
  content: React.ReactNode
  children: React.ReactNode
  side?: "top" | "right" | "bottom" | "left"
  align?: "start" | "center" | "end"
  delayDuration?: number
}

export function Tooltip({
  content,
  children,
  side = "top",
  align = "center",
  delayDuration = 300,
}: TooltipProps) {
  const [visible, setVisible] = React.useState(false)
  const timeoutRef = React.useRef<NodeJS.Timeout>()

  const showTooltip = () => {
    timeoutRef.current = setTimeout(() => setVisible(true), delayDuration)
  }

  const hideTooltip = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }
    setVisible(false)
  }

  return (
    <div
      className="relative inline-block"
      onMouseEnter={showTooltip}
      onMouseLeave={hideTooltip}
      onFocus={showTooltip}
      onBlur={hideTooltip}
    >
      {children}
      {visible && (
        <div
          className={cn(
            "absolute z-50 px-2 py-1 text-xs text-primary-foreground bg-primary rounded shadow-md whitespace-nowrap",
            "animate-in fade-in-0 zoom-in-95",
            side === "top" && "bottom-full mb-1 left-1/2 -translate-x-1/2",
            side === "bottom" && "top-full mt-1 left-1/2 -translate-x-1/2",
            side === "left" && "right-full mr-1 top-1/2 -translate-y-1/2",
            side === "right" && "left-full ml-1 top-1/2 -translate-y-1/2",
            align === "start" && (side === "top" || side === "bottom") && "-translate-x-0 left-0",
            align === "end" && (side === "top" || side === "bottom") && "-translate-x-full right-0"
          )}
        >
          {content}
        </div>
      )}
    </div>
  )
}

// Simple wrapper for Tippy-like behavior using CSS
export function TooltipSimple({ title, children, className }: { title: string; children: React.ReactNode; className?: string }) {
  return (
    <div className={cn("relative group inline-block", className)}>
      {children}
      <div className="absolute z-50 px-2 py-1 text-xs text-primary-foreground bg-primary rounded shadow-md whitespace-nowrap opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-opacity duration-150 bottom-full left-1/2 -translate-x-1/2 mb-1">
        {title}
      </div>
    </div>
  )
}