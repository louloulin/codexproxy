import { BrowserRouter, Routes, Route } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"

import { CodexPage } from "@/components/codex/CodexPage"
import { DashboardPage } from "@/components/dashboard/DashboardPage"
import { AccountPage } from "@/components/account/AccountPage"
import { LogsPage } from "@/components/logs/LogsPage"
import { ModelsPage } from "@/components/models/ModelsPage"
import { ProvidersPage } from "@/components/providers/ProvidersPage"
import { UsersPage } from "@/pages/UsersPage"
import { Layout } from "@/components/layout/Layout"
import { ToastProvider } from "@/components/ui/toast"
import { ErrorBoundary } from "@/components/ui/error-boundary"
import { ProtectedRoute } from "@/components/common/ProtectedRoute"
import { AuthProvider } from "@/contexts/AuthContext"
import { LoginPage } from "@/pages/LoginPage"
import "./i18n"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30000,
      retry: 1,
    },
  },
})

function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          <AuthProvider>
            <BrowserRouter>
              <Routes>
                {/* Public routes */}
                <Route path="/login" element={<LoginPage />} />
                
                {/* Protected routes with Layout */}
                <Route
                  path="/"
                  element={
                    <ProtectedRoute>
                      <Layout>
                        <Routes>
                          <Route path="/" element={<DashboardPage />} />
                          <Route path="/dashboard" element={<DashboardPage />} />
                          <Route path="/codex" element={<CodexPage />} />
                          <Route path="/account" element={<AccountPage />} />
                          <Route path="/logs" element={<LogsPage />} />
                          <Route path="/models" element={<ModelsPage />} />
                          <Route path="/providers" element={<ProvidersPage />} />
                          <Route path="/users" element={<UsersPage />} />
                        </Routes>
                      </Layout>
                    </ProtectedRoute>
                  }
                />
              </Routes>
            </BrowserRouter>
          </AuthProvider>
        </ToastProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}

export default App