"use client";

import { usePathname } from "next/navigation";
import Link from "next/link";
import {
  BookOpen,
  Terminal,
  Settings,
  Layers,
  MonitorSmartphone,
  Boxes,
  ChevronRight,
  Menu,
  X,
} from "lucide-react";
import { useState } from "react";

interface NavItem {
  title: string;
  href: string;
  icon?: React.ElementType;
  comingSoon?: boolean;
}

interface NavSection {
  title: string;
  items: NavItem[];
}

const navigation: NavSection[] = [
  {
    title: "Getting Started",
    items: [
      { title: "Introduction", href: "/docs", icon: BookOpen },
      {
        title: "Installation",
        href: "/docs/getting-started",
        icon: Terminal,
      },
    ],
  },
  {
    title: "Usage",
    items: [
      { title: "CLI Reference", href: "/docs/cli-reference", icon: Terminal },
      {
        title: "Configuration",
        href: "/docs/configuration",
        icon: Settings,
      },
    ],
  },
  {
    title: "Features",
    items: [
      { title: "Traffic Inspector", href: "/docs/features", icon: Layers },
      {
        title: "Desktop App",
        href: "/docs/desktop-app",
        icon: MonitorSmartphone,
      },
    ],
  },
  {
    title: "Internals",
    items: [
      {
        title: "Architecture",
        href: "/docs/architecture",
        icon: Boxes,
      },
    ],
  },
];

function NavLink({ item, pathname }: { item: NavItem; pathname: string }) {
  const isActive = pathname === item.href;
  const Icon = item.icon;

  return (
    <Link
      href={item.href}
      className={`flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors ${
        isActive
          ? "bg-violet-primary/10 text-violet-primary font-medium"
          : "text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200"
      }`}
    >
      {Icon && <Icon className="h-4 w-4 shrink-0" />}
      {item.title}
      {isActive && <ChevronRight className="ml-auto h-3 w-3" />}
    </Link>
  );
}

export function Sidebar() {
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <>
      {/* Mobile toggle */}
      <button
        type="button"
        onClick={() => setMobileOpen(true)}
        className="fixed top-4 left-4 z-50 rounded-lg border border-zinc-800 bg-zinc-900 p-2 lg:hidden"
        aria-label="Open navigation"
      >
        <Menu className="h-5 w-5" />
      </button>

      {/* Mobile overlay */}
      {mobileOpen && (
        <div
          role="button"
          tabIndex={0}
          aria-label="Close navigation"
          className="fixed inset-0 z-40 bg-black/60 lg:hidden"
          onClick={() => setMobileOpen(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setMobileOpen(false);
          }}
        />
      )}

      {/* Sidebar */}
      <aside
        className={`fixed top-0 left-0 z-50 flex h-full w-64 flex-col border-r border-zinc-800 bg-zinc-950 transition-transform lg:translate-x-0 ${
          mobileOpen ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        {/* Header */}
        <div className="flex h-16 items-center justify-between border-b border-zinc-800 px-4">
          <Link href="/" className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-md bg-violet-primary text-xs font-bold text-white">
              PZ
            </div>
            <span className="text-sm font-semibold">PortZero Docs</span>
          </Link>
          <button
            type="button"
            onClick={() => setMobileOpen(false)}
            className="rounded-lg p-1 text-zinc-400 hover:text-white lg:hidden"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Nav */}
        <nav className="flex-1 overflow-y-auto p-4">
          {navigation.map((section) => (
            <div key={section.title} className="mb-6">
              <h4 className="mb-2 px-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">
                {section.title}
              </h4>
              <div className="space-y-0.5">
                {section.items.map((item) => (
                  <NavLink
                    key={item.href}
                    item={item}
                    pathname={pathname}
                  />
                ))}
              </div>
            </div>
          ))}
        </nav>

        {/* Footer */}
        <div className="border-t border-zinc-800 p-4">
          <a
            href="https://github.com/portzero-dev/portzero"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-xs text-zinc-500 transition-colors hover:text-zinc-300"
          >
            GitHub
            <span className="text-zinc-700">|</span>
            MIT / Apache-2.0
          </a>
        </div>
      </aside>
    </>
  );
}
