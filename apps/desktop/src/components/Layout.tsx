import type { ReactNode } from "react";
import type { Route } from "../App";
import {
  LayoutDashboard,
  Activity,
  Box,
  Settings,
  Wifi,
  WifiOff,
} from "lucide-react";

interface LayoutProps {
  route: Route;
  navigate: (route: Route) => void;
  connected: boolean;
  children: ReactNode;
}

interface NavItem {
  label: string;
  page: Route["page"];
  icon: ReactNode;
  route: Route;
}

const navItems: NavItem[] = [
  {
    label: "Overview",
    page: "overview",
    icon: <LayoutDashboard size={18} />,
    route: { page: "overview" },
  },
  {
    label: "Traffic",
    page: "traffic",
    icon: <Activity size={18} />,
    route: { page: "traffic" },
  },
  {
    label: "Mocks",
    page: "mocks",
    icon: <Box size={18} />,
    route: { page: "mocks" },
  },
  {
    label: "Settings",
    page: "settings",
    icon: <Settings size={18} />,
    route: { page: "settings" },
  },
];

export function Layout({ route, navigate, connected, children }: LayoutProps) {
  const isActive = (item: NavItem) => {
    if (item.page === "traffic" && (route.page === "traffic" || route.page === "request" || route.page === "diff")) {
      return true;
    }
    return route.page === item.page;
  };

  return (
    <div className="flex h-screen bg-zinc-950">
      {/* Sidebar */}
      <aside className="flex w-56 flex-col border-r border-zinc-800 bg-zinc-900">
        {/* Logo */}
        <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-4">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-600">
            <span className="text-sm font-bold text-white">PZ</span>
          </div>
          <span className="text-lg font-semibold text-zinc-100">PortZero</span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 px-2 py-3">
          {navItems.map((item) => (
            <button
              type="button"
              key={item.page}
              onClick={() => navigate(item.route)}
              className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                isActive(item)
                  ? "bg-zinc-800 text-zinc-100"
                  : "text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200"
              }`}
            >
              {item.icon}
              {item.label}
            </button>
          ))}
        </nav>

        {/* Connection Status */}
        <div className="border-t border-zinc-800 px-4 py-3">
          <div className="flex items-center gap-2 text-xs">
            {connected ? (
              <>
                <Wifi size={14} className="text-emerald-400" />
                <span className="text-zinc-400">Connected</span>
              </>
            ) : (
              <>
                <WifiOff size={14} className="text-red-400" />
                <span className="text-zinc-400">Disconnected</span>
              </>
            )}
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">{children}</main>
    </div>
  );
}
