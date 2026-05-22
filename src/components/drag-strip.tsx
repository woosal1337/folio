import * as React from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { cn } from "@/lib/utils";

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

  return (
    <div
      data-tauri-drag-region
      data-drag=""
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
      className={cn("h-9 w-full shrink-0 select-none bg-sidebar", className)}
    />
  );
}
