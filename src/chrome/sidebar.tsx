import * as React from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  AudioLines,
  KanbanSquare,
  Library,
  Moon,
  Settings as SettingsIcon,
  Sun,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/hooks/use-theme";
import { Button } from "@/shared/ui/button";
import logoUrl from "@/assets/logo.svg";

interface NavItem {
  to: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  /** Extra path prefixes that should also light this item up. The
   * editor route is a per-recording detail page reached from Library;
   * NavLink's default isActive only matches the exact `to`, which left
   * the sidebar feeling unmoored on /editor/*. Including the prefix
   * here makes Library stay highlighted while the user is inside a
   * recording. */
  alsoActiveOn?: string[];
}

const items: NavItem[] = [
  { to: "/record", label: "Record", icon: AudioLines },
  { to: "/library", label: "Library", icon: Library, alsoActiveOn: ["/editor"] },
  { to: "/tasks", label: "Tasks", icon: KanbanSquare },
];

interface SidebarProps {
  onOpenSettings: () => void;
}

export function Sidebar({ onOpenSettings }: SidebarProps) {
  const { theme, toggle } = useTheme();
  const location = useLocation();

  return (
    <aside
      data-drag=""
      className="flex w-[220px] select-none flex-col border-r border-border bg-sidebar text-sidebar-foreground"
    >
      {/* Brand */}
      <div className="flex items-center gap-2.5 px-5 pb-2 pt-4">
        <img
          src={logoUrl}
          alt="Attune"
          className="h-6 w-6 select-none"
          draggable={false}
        />
        <span className="font-serif text-2xl font-medium tracking-tight">attune</span>
      </div>
      <div className="pb-4" />

      {/* Primary nav */}
      <nav className="flex-1 space-y-0.5 px-2">
        {items.map((item) => {
          const Icon = item.icon;
          const alsoActive = item.alsoActiveOn?.some((prefix) =>
            location.pathname.startsWith(prefix)
          );
          return (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                cn(
                  "group flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  isActive || alsoActive
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                )
              }
            >
              <Icon className="h-4 w-4 shrink-0" />
              <span>{item.label}</span>
            </NavLink>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="flex flex-col gap-1 border-t border-border px-2 py-3">
        <Button
          variant="ghost"
          size="sm"
          className="justify-start gap-3 font-medium text-muted-foreground"
          onClick={onOpenSettings}
        >
          <SettingsIcon className="h-4 w-4" />
          Settings
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="justify-start gap-3 font-medium text-muted-foreground"
          onClick={toggle}
          aria-label="Toggle theme"
        >
          {theme === "light" ? (
            <Moon className="h-4 w-4" />
          ) : (
            <Sun className="h-4 w-4" />
          )}
          {theme === "light" ? "Dark mode" : "Light mode"}
        </Button>
        <div className="mt-2 px-3 pb-1 text-2xs text-muted-foreground">
          v1.0.0 · audio stays on this Mac
        </div>
      </div>
    </aside>
  );
}
