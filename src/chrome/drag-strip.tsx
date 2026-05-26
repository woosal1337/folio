import * as React from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ShieldCheck } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useSettingsStore } from "@/shared/stores/settings-store";

/** A full-width drag handle that lives at the top of the window. macOS
 *  traffic lights overlay this region at the system level and stay
 *  clickable. Both `data-tauri-drag-region` and an explicit `startDragging`
 *  handler are wired so dragging works regardless of which path the
 *  current Tauri/macOS combo prefers. */
export function DragStrip({ className }: { className?: string }) {
  const handleMouseDown = React.useCallback(
    async (e: React.MouseEvent<HTMLDivElement>) => {
      // Left button only.
      if (e.button !== 0) return;
      // Don't hijack clicks on interactive children (we have none in this
      // strip today, but stay defensive in case we add a topbar later).
      const target = e.target as HTMLElement;
      if (target.closest("button, a, input, select, textarea, [role='button']")) {
        return;
      }
      try {
        await getCurrentWindow().startDragging();
      } catch (err) {
        console.error("startDragging:", err);
      }
    },
    []
  );

  const handleDoubleClick = React.useCallback(async () => {
    try {
      const win = getCurrentWindow();
      const maximized = await win.isMaximized();
      if (maximized) await win.unmaximize();
      else await win.maximize();
    } catch (err) {
      console.error("toggle maximize:", err);
    }
  }, []);

  // Mirror the privacy_mode flag from settings + Rust-side
  // `privacy-mode-changed` events. The badge appears in the titlebar
  // whenever the CloudGuard is actively blocking egress. v2 finding
  // 048 / GET-42.
  const privacyMode = useSettingsStore((s) => s.settings?.privacy_mode ?? false);
  const [eventMode, setEventMode] = React.useState<boolean | null>(null);
  React.useEffect(() => {
    let off: UnlistenFn | undefined;
    listen<boolean>("privacy-mode-changed", (ev) => {
      setEventMode(ev.payload);
    }).then((unlisten) => {
      off = unlisten;
    });
    return () => {
      off?.();
    };
  }, []);
  const airgap = eventMode ?? privacyMode;

  return (
    <div
      data-tauri-drag-region
      data-drag=""
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
      className={cn(
        "relative flex h-9 w-full shrink-0 select-none items-center justify-center bg-sidebar",
        className
      )}
    >
      {airgap ? (
        <span
          className="pointer-events-none inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-2xs font-medium uppercase tracking-wider text-emerald-700 ring-1 ring-emerald-500/30 dark:text-emerald-300"
          title="Privacy Mode is on — all cloud egress is blocked. Local-only services (localhost) still work."
        >
          <ShieldCheck className="h-3 w-3" />
          Airgap
        </span>
      ) : null}
    </div>
  );
}
