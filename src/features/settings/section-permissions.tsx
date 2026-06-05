import * as React from "react";
import { ExternalLink, Bell, Calendar, ListChecks, Mic, Monitor } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Badge } from "@/shared/ui/badge";
import { listPermissions, openPermissionSettings } from "@/shared/lib/ipc";
import type { PermissionRow } from "@/shared/types/PermissionRow";
import type { Permission } from "@/shared/types/Permission";
import type { PermissionStatus } from "@/shared/types/PermissionStatus";

const ICONS: Record<Permission, React.ComponentType<{ className?: string }>> = {
  microphone: Mic,
  screen_recording: Monitor,
  calendar: Calendar,
  reminders: ListChecks,
  notifications: Bell,
};

const LABELS: Record<Permission, string> = {
  microphone: "Microphone",
  screen_recording: "Screen Recording",
  calendar: "Calendar",
  reminders: "Reminders",
  notifications: "Notifications",
};

export function SectionPermissions() {
  const [rows, setRows] = React.useState<PermissionRow[]>([]);
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const next = await listPermissions();
        if (!cancelled) {
          setRows(next);
          setLoading(false);
        }
      } catch (e) {
        if (!cancelled) {
          console.error("list_permissions:", e);
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex flex-col gap-7">
      <div>
        <h2 className="font-serif text-2xl font-medium">Permissions</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Empty or blocked permissions are the silent killer of first-recording. Tap
          Open Settings to grant access in macOS System Settings; the change shows up
          here next time you open this pane.
        </p>
      </div>

      <ul className="flex flex-col gap-3">
        {rows.map((row) => {
          const Icon = ICONS[row.permission];
          return (
            <li
              key={row.permission}
              className="flex items-start gap-3 rounded-lg border border-border bg-card p-4"
            >
              <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium">{LABELS[row.permission]}</p>
                  <StatusBadge status={row.status} />
                </div>
                <p className="mt-1 text-xs text-muted-foreground">{row.rationale}</p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  openPermissionSettings(row.permission).catch((e) =>
                    console.error("open_permission_settings:", e)
                  );
                }}
                className="shrink-0 gap-1"
                aria-label={`Open ${LABELS[row.permission]} settings`}
              >
                <ExternalLink className="h-3 w-3" />
                Settings
              </Button>
            </li>
          );
        })}
        {loading && rows.length === 0 ? (
          <li className="text-xs text-muted-foreground">Loading…</li>
        ) : null}
      </ul>
    </div>
  );
}

function StatusBadge({ status }: { status: PermissionStatus }) {
  switch (status) {
    case "granted":
      return (
        <Badge variant="accent" className="text-2xs">
          Granted
        </Badge>
      );
    case "denied":
      return (
        <Badge variant="destructive" className="text-2xs">
          Denied
        </Badge>
      );
    case "not_determined":
      return (
        <Badge variant="outline" className="text-2xs">
          Not asked
        </Badge>
      );
    case "restricted":
      return (
        <Badge variant="destructive" className="text-2xs">
          Restricted
        </Badge>
      );
    case "unknown":
    default:
      return (
        <Badge variant="outline" className="text-2xs">
          Unknown
        </Badge>
      );
  }
}
