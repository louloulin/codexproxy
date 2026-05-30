import { createContext, useContext, useState, useEffect, ReactNode } from "react"
import { api } from "@/lib/api"

interface User {
  id: number
  username: string
  role: string
  is_admin: boolean
  created_at: number
}

interface AuthContextType {
  user: User | null
  token: string | null
  isLoading: boolean
  login: (username: string, password: string) => Promise<{ success: boolean; error?: string }>
  logout: () => Promise<void>
  isAuthenticated: boolean
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

const TOKEN_KEY = "rcodex_auth_token"
const USER_KEY = "rcodex_auth_user"

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  // Load token and user from localStorage on mount
  useEffect(() => {
    const savedToken = localStorage.getItem(TOKEN_KEY)
    const savedUser = localStorage.getItem(USER_KEY)
    
    if (savedToken && savedUser) {
      try {
        setToken(savedToken)
        setUser(JSON.parse(savedUser))
        // Verify token is still valid by calling /auth/me
        verifyToken(savedToken)
      } catch {
        // Invalid stored data, clear it
        localStorage.removeItem(TOKEN_KEY)
        localStorage.removeItem(USER_KEY)
        setIsLoading(false)
      }
    } else {
      setIsLoading(false)
    }
  }, [])

  async function verifyToken(t: string) {
    try {
      const response = await api.auth.me()
      if (response.ok && response.data) {
        setUser(response.data)
        setToken(t)
      } else {
        // Token invalid, clear storage
        localStorage.removeItem(TOKEN_KEY)
        localStorage.removeItem(USER_KEY)
        setUser(null)
        setToken(null)
      }
    } catch {
      // API call failed, assume token invalid
      localStorage.removeItem(TOKEN_KEY)
      localStorage.removeItem(USER_KEY)
      setUser(null)
      setToken(null)
    } finally {
      setIsLoading(false)
    }
  }

  async function login(username: string, password: string): Promise<{ success: boolean; error?: string }> {
    try {
      const response = await api.auth.login(username, password)
      
      if (response.ok && response.data) {
        const { token: newToken, user: newUser } = response.data
        
        // Store in localStorage
        localStorage.setItem(TOKEN_KEY, newToken)
        localStorage.setItem(USER_KEY, JSON.stringify(newUser))
        
        setToken(newToken)
        setUser(newUser)
        
        return { success: true }
      } else {
        return { success: false, error: response.error || "Login failed" }
      }
    } catch (err) {
      return { success: false, error: err instanceof Error ? err.message : "Network error" }
    }
  }

  async function logout() {
    try {
      await api.auth.logout()
    } catch {
      // Ignore logout errors
    } finally {
      localStorage.removeItem(TOKEN_KEY)
      localStorage.removeItem(USER_KEY)
      setToken(null)
      setUser(null)
    }
  }

  return (
    <AuthContext.Provider
      value={{
        user,
        token,
        isLoading,
        login,
        logout,
        isAuthenticated: !!token && !!user,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider")
  }
  return context
}