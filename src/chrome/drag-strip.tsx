import * as React from "react";
import { ShieldCheck } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import {
  onPrivacyModeChanged,
  startWindowDrag,
  toggleWindowMaximize,
} from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";

export function DragStrip({ className }: { className?: string }) {
  const handleMouseDown = React.useCallback(
    async (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;

      const target = e.target as HTMLElement;
      if (target.closest("button, a, input, select, textarea, [role='button']")) {
        return;
      }
      try {
        await startWindowDrag();
      } catch (err) {
        console.error("startWindowDrag:", err);
      }
    },
    []
  );

  const handleDoubleClick = React.useCallback(async () => {
    try {
      await toggleWindowMaximize();
    } catch (err) {
      console.error("toggleWindowMaximize:", err);
    }
  }, []);

  const privacyMode = useSettingsStore((s) => s.settings?.privacy_mode ?? false);
  const [eventMode, setEventMode] = React.useState<boolean | null>(null);
  React.useEffect(() => {
    let off: (() => void) | undefined;
    let cancelled = false;
    (async () => {
      const unlisten = await onPrivacyModeChanged((enabled) => setEventMode(enabled));
      if (cancelled) unlisten();
      else off = unlisten;
    })();
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);
  const airgap = eventMode ?? privacyMode;

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions
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
