import * as React from "react";
import { NavLink } from "react-router-dom";
import {
  AudioLines,
  Library,
  Pencil,
  KanbanSquare,
  Settings as SettingsIcon,
  Moon,
  Sun,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useTheme } from "@/hooks/use-theme";
import { Button } from "@/components/ui/button";

interface NavItem {
  to: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const items: NavItem[] = [
  { to: "/record", label: "Record", icon: AudioLines },
  { to: "/library", label: "Library", icon: Library },
  { to: "/editor", label: "Editor", icon: Pencil },
  { to: "/tasks", label: "Tasks", icon: KanbanSquare },
];

interface SidebarProps {
  onOpenSettings: () => void;
}

export function Sidebar({ onOpenSettings }: SidebarProps) {
  const { theme, toggle } = useTheme();

  return (
    <aside
      data-drag=""
      className="flex w-[220px] flex-col select-none border-r border-border bg-sidebar text-sidebar-foreground"
    >
      {/* Brand */}
      <div className="flex items-center gap-2 px-5 pb-2 pt-4">
        <span className="font-serif text-2xl font-medium tracking-tight">
          attune
        </span>
      </div>
      <div className="px-5 pb-4">
        <p className="text-xs text-muted-foreground">local meeting capture</p>
      </div>

      {/* Primary nav */}
      <nav className="flex-1 space-y-0.5 px-2">
        {items.map((item) => {
          const Icon = item.icon;
          return (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                cn(
                  "group flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  isActive
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
          v0.0.1 · audio stays on this Mac
        </div>
      </div>
    </aside>
  );
}
