import { NavLink } from "react-router-dom"
import {
  LayoutDashboard,
  Settings,
  Server,
  Cpu,
  FileText,
  UserCog,
  Users,
  Key,
  Zap,
} from "lucide-react"

const navItems = [
  { to: "/dashboard", icon: LayoutDashboard, label: "Dashboard" },
  { to: "/codex", icon: Settings, label: "Codex" },
  { to: "/providers", icon: Server, label: "Providers" },
  { to: "/models", icon: Cpu, label: "Models" },
  { to: "/logs", icon: FileText, label: "Logs" },
  { to: "/users", icon: Users, label: "Users" },
  { to: "/account", icon: UserCog, label: "Account" },
  { to: "/settings", icon: Settings, label: "Settings" },
]

export function Sidebar() {
  return (
    <aside className="fixed left-0 top-0 bottom-0 w-60 bg-card border-r border-border flex flex-col z-10">
      {/* Brand */}
      <div className="px-4 py-4 border-b border-border flex items-center gap-2">
        <Zap className="h-5 w-5 text-primary" />
        <span className="font-bold text-lg">rcodex</span>
        <span className="text-xs bg-primary text-primary-foreground px-1.5 py-0.5 rounded">
          admin
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-3 space-y-1">
        <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold px-3 pt-4 pb-2">
          Overview
        </div>
        {navItems.slice(0, 1).map((item) => (
          <NavItem key={item.to} {...item} />
        ))}

        <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold px-3 pt-4 pb-2">
          Management
        </div>
        {navItems.slice(1, 5).map((item) => (
          <NavItem key={item.to} {...item} />
        ))}

        <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold px-3 pt-4 pb-2">
          Settings
        </div>
        {navItems.slice(5).map((item) => (
          <NavItem key={item.to} {...item} />
        ))}
      </nav>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-border text-xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <Key className="h-3.5 w-3.5" />
          <span>v1.0.0</span>
        </div>
      </div>
    </aside>
  )
}

interface NavItemProps {
  to: string
  icon: React.ElementType
  label: string
}

function NavItem({ to, icon: Icon, label }: NavItemProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
          isActive
            ? "bg-primary text-primary-foreground"
            : "text-muted-foreground hover:bg-muted hover:text-foreground"
        }`
      }
    >
      <Icon className="h-4 w-4" />
      <span>{label}</span>
    </NavLink>
  )
}