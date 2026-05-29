import { cn } from "@/lib/utils"

interface AvatarProps {
  src?: string
  alt?: string
  fallback?: string
  className?: string
  size?: "sm" | "md" | "lg"
}

export function Avatar({ src, alt, fallback, className, size = "md" }: AvatarProps) {
  const [error, setError] = React.useState(false)
  const sizeClasses = {
    sm: "h-6 w-6 text-xs",
    md: "h-8 w-8 text-sm",
    lg: "h-10 w-10 text-base",
  }

  return (
    <div
      className={cn(
        "relative inline-flex items-center justify-center rounded-full bg-muted overflow-hidden",
        sizeClasses[size],
        className
      )}
    >
      {src && !error ? (
        <img
          src={src}
          alt={alt || "Avatar"}
          className="h-full w-full object-cover"
          onError={() => setError(true)}
        />
      ) : (
        <span className="font-medium text-muted-foreground">
          {fallback || "?"}
        </span>
      )}
    </div>
  )
}

// AvatarGroup for showing multiple avatars
export function AvatarGroup({
  avatars,
  max = 4,
  size = "sm",
}: {
  avatars: Array<{ src?: string; alt?: string; fallback?: string }>
  max?: number
  size?: "sm" | "md" | "lg"
}) {
  const shown = avatars.slice(0, max)
  const remaining = avatars.length - max

  return (
    <div className="flex -space-x-2">
      {shown.map((avatar, i) => (
        <Avatar
          key={i}
          src={avatar.src}
          alt={avatar.alt}
          fallback={avatar.fallback}
          size={size}
          className="ring-2 ring-background"
        />
      ))}
      {remaining > 0 && (
        <div
          className={cn(
            "relative inline-flex items-center justify-center rounded-full bg-muted-foreground text-muted foreground ring-2 ring-background font-medium",
            size === "sm" && "h-6 w-6 text-xs",
            size === "md" && "h-8 w-8 text-xs",
            size === "lg" && "h-10 w-10 text-sm"
          )}
        >
          +{remaining}
        </div>
      )}
    </div>
  )
}

// Import React for useState
import * as React from "react"