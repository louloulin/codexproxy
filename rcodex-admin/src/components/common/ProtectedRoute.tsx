import { useEffect, useState } from "react"
import { Navigate, useLocation } from "react-router-dom"
import { useAuth } from "@/contexts/AuthContext"
import { api } from "@/lib/api"
import { Loader2 } from "lucide-react"

interface ProtectedRouteProps {
  children: React.ReactNode
}

export function ProtectedRoute({ children }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading } = useAuth()
  const location = useLocation()
  const [bootstrapChecked, setBootstrapChecked] = useState(false)
  const [needsBootstrap, setNeedsBootstrap] = useState(false)

  // Check bootstrap status when not authenticated
  useEffect(() => {
    if (isLoading) return

    async function checkBootstrap() {
      try {
        const status = await api.bootstrap.status()
        setNeedsBootstrap(status.needsBootstrap)
      } catch (e) {
        console.error("Failed to check bootstrap status:", e)
      } finally {
        setBootstrapChecked(true)
      }
    }

    if (!isAuthenticated) {
      checkBootstrap()
    } else {
      setBootstrapChecked(true)
    }
  }, [isAuthenticated, isLoading])

  if (isLoading || !bootstrapChecked) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    )
  }

  if (!isAuthenticated) {
    // If no users exist, redirect to bootstrap
    if (needsBootstrap) {
      return <Navigate to="/bootstrap" state={{ from: location }} replace />
    }
    // Otherwise redirect to login
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  return <>{children}</>
}