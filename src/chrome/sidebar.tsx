import * as React from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  BarChart2,
  CircleUserRound,
  Home as HomeIcon,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Settings as SettingsIcon,
  Sun,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/hooks/use-theme";
import { useSidebarCollapsed } from "@/shared/hooks/use-sidebar-collapsed";
import { useRemoteAccountStore } from "@/shared/stores/remote-account-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { Button } from "@/shared/ui/button";
import logoUrl from "@/assets/logo.svg";

interface NavItem {
  to: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;

  alsoActiveOn?: string[];
}

const items: NavItem[] = [
  { to: "/", label: "Home", icon: HomeIcon, alsoActiveOn: ["/editor"] },
  { to: "/stats", label: "Stats", icon: BarChart2 },
];

interface SidebarProps {
  onOpenSettings: () => void;
}

export function Sidebar({ onOpenSettings }: SidebarProps) {
  const { theme, toggle: toggleTheme } = useTheme();
  const { collapsed, toggle: toggleCollapsed } = useSidebarCollapsed();
  const location = useLocation();
  const account = useRemoteAccountStore((s) => s.account);
  const refreshAccount = useRemoteAccountStore((s) => s.refresh);
  const remoteActive = useSettingsStore(
    (s) =>
      s.settings?.transcriber === "remote_server" &&
      (s.settings?.remote_endpoint ?? "").trim().length > 0
  );

  React.useEffect(() => {
    void refreshAccount();
  }, [refreshAccount]);

  const accountActive = location.pathname.startsWith("/account");

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
      <div
        className={cn(
          "flex w-full items-center pt-4",
          collapsed ? "flex-col gap-2 px-2 pb-2" : "justify-between gap-2 px-5 pb-2"
        )}
      >
        <div className={cn("flex items-center gap-2.5", collapsed && "justify-center")}>
          <img
            src={logoUrl}
            alt="Folio"
            className="h-6 w-6 select-none"
            draggable={false}
          />
          {!collapsed && (
            <span className="font-wordmark text-2xl font-medium tracking-tight">
              folio
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

      <div className="flex-1 overflow-y-auto">
        <nav className={cn("space-y-0.5", collapsed ? "w-full px-1.5" : "px-2")}>
          {items.map((item) => {
            const Icon = item.icon;
            const alsoActive = item.alsoActiveOn?.some((prefix) =>
              location.pathname.startsWith(prefix)
            );
            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === "/"}
                title={collapsed ? item.label : undefined}
                aria-label={collapsed ? item.label : undefined}
                className={({ isActive }) =>
                  cn(
                    "group flex items-center text-sm font-medium transition-colors",

                    collapsed
                      ? "mx-auto h-9 w-9 justify-center rounded-xl"
                      : "gap-3 rounded-md px-3 py-2",
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
      </div>

      <div
        className={cn(
          "flex w-full flex-col gap-1 border-t border-border py-3",
          collapsed ? "items-center px-1.5" : "px-2"
        )}
      >
        <NavLink
          to="/account"
          aria-label="Account"
          title={collapsed ? "Account" : undefined}
          className={cn(
            "inline-flex items-center rounded-md text-sm font-medium transition-colors",
            collapsed ? "h-9 w-9 justify-center" : "h-8 justify-start gap-3 px-3",
            accountActive
              ? "bg-accent text-accent-foreground"
              : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
          )}
        >
          <CircleUserRound className="h-4 w-4 shrink-0" />
          {!collapsed && (
            <span className="min-w-0 flex-1 truncate text-left">
              {account?.signed_in ? (account.email ?? "Account") : "Account"}
            </span>
          )}
          {!collapsed && account?.signed_in ? (
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500"
              title="Signed in to your server"
              aria-label="Signed in to your server"
            />
          ) : null}
        </NavLink>
        <Button
          variant="ghost"
          size={collapsed ? "icon" : "sm"}
          className={cn(
            "text-sm font-medium text-muted-foreground",
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
            "text-sm font-medium text-muted-foreground",
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
            v{__FOLIO_VERSION__} ·{" "}
            {remoteActive ? "synced to your server" : "audio stays on this Mac"}
          </div>
        )}
      </div>
    </aside>
  );
}
