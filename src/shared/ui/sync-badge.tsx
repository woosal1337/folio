import { Cloud, CloudOff, Loader2 } from "lucide-react";

import type { SyncState } from "@/shared/types/SyncState";

export function SyncBadge({ sync }: { sync: SyncState }) {
  if (sync.remote_status === "succeeded") {
    return (
      <span
        className="inline-flex items-center gap-1 text-2xs text-sky-600 dark:text-sky-400"
        title="Uploaded to your server and transcript synced back"
      >
        <Cloud className="h-3 w-3" />
        Synced
      </span>
    );
  }
  if (sync.remote_status === "failed") {
    return (
      <span
        className="inline-flex items-center gap-1 text-2xs text-red-600 dark:text-red-400"
        title={sync.error ?? "Sync failed"}
      >
        <CloudOff className="h-3 w-3" />
        Sync failed
      </span>
    );
  }
  const label =
    sync.upload_state !== "complete"
      ? "Uploading"
      : sync.remote_status === "running"
        ? "On GPU"
        : "Queued";
  return (
    <span className="inline-flex items-center gap-1 text-2xs text-muted-foreground">
      <Loader2 className="h-3 w-3 animate-spin" />
      {label}
    </span>
  );
}
