import * as React from "react";

import { startWindowDrag, toggleWindowMaximize } from "@/shared/lib/ipc";

export function useWindowDrag() {
  return React.useCallback(async (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (!target) return;

    if (
      target.closest(
        "button, a, input, select, textarea, [contenteditable=''], [contenteditable='true'], [role='button'], [data-no-drag]"
      )
    ) {
      return;
    }

    if (!target.closest("[data-drag]")) {
      return;
    }

    e.preventDefault();
    try {
      await startWindowDrag();
    } catch (err) {
      console.error("startWindowDrag failed:", err);
    }
  }, []);
}

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
      await toggleWindowMaximize();
    } catch (err) {
      console.error("toggleWindowMaximize failed:", err);
    }
  }, []);
}
