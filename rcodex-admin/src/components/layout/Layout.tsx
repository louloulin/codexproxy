import { ReactNode } from "react"
import { Sidebar } from "./Sidebar"
import { Header } from "./Header"
import { UpdateBanner } from "@/components/common/UpdateBanner"

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col ml-60">
        <Header />
        <UpdateBanner />
        <main className="flex-1 p-6">
          {children}
        </main>
      </div>
    </div>
  )
}