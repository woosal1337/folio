import * as React from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  AudioLines,
  Brain,
  KanbanSquare,
  Library,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Settings as SettingsIcon,
  Sun,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/hooks/use-theme";
import { useSidebarCollapsed } from "@/shared/hooks/use-sidebar-collapsed";
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
  // /ai (the flat reverse-chronological agent-runs page) was retired
  // by v2 finding R03 / GET-120 because the agent panel inside the
  // editor (the run-cards inbox) subsumed it and /Chat will take its
  // slot when #021 ships. The route stays alive for now redirecting
  // to /library, so deep-links from prior versions still land
  // somewhere useful.
  { to: "/tasks", label: "Tasks", icon: KanbanSquare },
  { to: "/memory", label: "Memory", icon: Brain },
];

interface SidebarProps {
  onOpenSettings: () => void;
}

export function Sidebar({ onOpenSettings }: SidebarProps) {
  const { theme, toggle: toggleTheme } = useTheme();
  const { collapsed, toggle: toggleCollapsed } = useSidebarCollapsed();
  const location = useLocation();

  return (
    <aside
      data-drag=""
      aria-label="Primary navigation"
      data-collapsed={collapsed || undefined}
      className={cn(
        "flex select-none flex-col border-r border-border bg-sidebar text-sidebar-foreground transition-[width] duration-150 ease-out",
        collapsed ? "w-[56px] items-center" : "w-[220px]"
      )}
    >
      {/* Brand + collapse toggle. In rail mode the wordmark hides and
          the toggle stays centered under the logo so the column reads
          as one stack of icons. */}
      <div
        className={cn(
          "flex w-full items-center pt-4",
          collapsed ? "flex-col gap-2 px-2 pb-2" : "justify-between gap-2 px-5 pb-2"
        )}
      >
        <div className={cn("flex items-center gap-2.5", collapsed && "justify-center")}>
          <img
            src={logoUrl}
            alt="Attune"
            className="h-6 w-6 select-none"
            draggable={false}
          />
          {!collapsed && (
            <span className="font-serif text-2xl font-medium tracking-tight">
              attune
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground"
          onClick={toggleCollapsed}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-pressed={collapsed}
          title={collapsed ? "Expand sidebar (⌘⌃S)" : "Collapse sidebar (⌘⌃S)"}
        >
          {collapsed ? (
            <PanelLeftOpen className="h-4 w-4" />
          ) : (
            <PanelLeftClose className="h-4 w-4" />
          )}
        </Button>
      </div>
      <div className="pb-4" />

      {/* Primary nav */}
      <nav className={cn("flex-1 space-y-0.5", collapsed ? "w-full px-1.5" : "px-2")}>
        {items.map((item) => {
          const Icon = item.icon;
          const alsoActive = item.alsoActiveOn?.some((prefix) =>
            location.pathname.startsWith(prefix)
          );
          return (
            <NavLink
              key={item.to}
              to={item.to}
              title={collapsed ? item.label : undefined}
              aria-label={collapsed ? item.label : undefined}
              className={({ isActive }) =>
                cn(
                  "group flex items-center rounded-md text-sm font-medium transition-colors",
                  collapsed ? "h-9 w-full justify-center" : "gap-3 px-3 py-2",
                  isActive || alsoActive
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                )
              }
            >
              <Icon className="h-4 w-4 shrink-0" />
              {!collapsed && <span>{item.label}</span>}
            </NavLink>
          );
        })}
      </nav>

      {/* Footer */}
      <div
        className={cn(
          "flex w-full flex-col gap-1 border-t border-border py-3",
          collapsed ? "items-center px-1.5" : "px-2"
        )}
      >
        <Button
          variant="ghost"
          size={collapsed ? "icon" : "sm"}
          className={cn(
            "font-medium text-muted-foreground",
            collapsed ? "h-9 w-9" : "justify-start gap-3"
          )}
          onClick={onOpenSettings}
          aria-label="Settings"
          title={collapsed ? "Settings" : undefined}
        >
          <SettingsIcon className="h-4 w-4" />
          {!collapsed && <span>Settings</span>}
        </Button>
        <Button
          variant="ghost"
          size={collapsed ? "icon" : "sm"}
          className={cn(
            "font-medium text-muted-foreground",
            collapsed ? "h-9 w-9" : "justify-start gap-3"
          )}
          onClick={toggleTheme}
          aria-label={
            theme === "light" ? "Switch to dark mode" : "Switch to light mode"
          }
          title={
            collapsed ? (theme === "light" ? "Dark mode" : "Light mode") : undefined
          }
        >
          {theme === "light" ? (
            <Moon className="h-4 w-4" />
          ) : (
            <Sun className="h-4 w-4" />
          )}
          {!collapsed && <span>{theme === "light" ? "Dark mode" : "Light mode"}</span>}
        </Button>
        {!collapsed && (
          <div className="mt-2 px-3 pb-1 text-2xs text-muted-foreground">
            v1.0.0 · audio stays on this Mac
          </div>
        )}
      </div>
    </aside>
  );
}
