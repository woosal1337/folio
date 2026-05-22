import * as React from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** A root-level `onMouseDown` handler that turns any element marked with
 *  `data-drag` into a window drag handle. Interactive elements inside a
 *  drag region (buttons, links, inputs, text areas) are excluded so they
 *  still receive their clicks; elements that opt out of dragging mark
 *  themselves with `data-no-drag`.
 *
 *  This complements `data-tauri-drag-region` (which sometimes silently
 *  fails on macOS with overlay title bar) with an explicit
 *  `startDragging()` call. */
export function useWindowDrag() {
  return React.useCallback(async (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (!target) return;

    // Don't hijack clicks on interactive children.
    if (
      target.closest(
        "button, a, input, select, textarea, [contenteditable=''], [contenteditable='true'], [role='button'], [data-no-drag]"
      )
    ) {
      return;
    }

    // Must be inside an explicitly draggable region.
    if (!target.closest("[data-drag]")) {
      return;
    }

    e.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch (err) {
      console.error("startDragging failed:", err);
    }
  }, []);
}

/** Double-click on a drag region toggles maximize, matching macOS behavior. */
export function useWindowDoubleClick() {
  return React.useCallback(async (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (
      target.closest(
        "button, a, input, select, textarea, [role='button'], [data-no-drag]"
      )
    ) {
      return;
    }
    if (!target.closest("[data-drag]")) {
      return;
    }
    try {
      const win = getCurrentWindow();
      const maximized = await win.isMaximized();
      if (maximized) await win.unmaximize();
      else await win.maximize();
    } catch (err) {
      console.error("toggle maximize failed:", err);
    }
  }, []);
}
