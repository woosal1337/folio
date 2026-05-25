import * as React from "react";

/**
 * Persistent collapsed/expanded state for the chrome sidebar.
 *
 * - Reads the user's preference from localStorage on mount.
 * - Auto-forces the rail mode when the window is narrower than the
 *   AUTO_COLLAPSE_BREAKPOINT, so the sidebar doesn't crowd content on
 *   small windows. The user preference is restored as soon as the
 *   window grows again.
 * - Exposes a Cmd+Ctrl+S keyboard shortcut to toggle. Matches Apple
 *   Mail's "Hide / Show Mailbox List" shortcut so the muscle memory
 *   carries over from the system app the sidebar imitates.
 *
 * v2 roadmap finding 015 (GET-49).
 */

const STORAGE_KEY = "attune.sidebar.collapsed";
const AUTO_COLLAPSE_BREAKPOINT = 900;

function readStored(): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(STORAGE_KEY) === "1";
}

export function useSidebarCollapsed() {
  const [userPref, setUserPref] = React.useState<boolean>(() => readStored());
  const [forcedByViewport, setForcedByViewport] = React.useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.innerWidth < AUTO_COLLAPSE_BREAKPOINT;
  });

  // Persist the user preference whenever it changes. The forced-by-viewport
  // state is derived and intentionally not persisted — when the window grows
  // back, we want the user's prior choice to take effect again.
  React.useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, userPref ? "1" : "0");
  }, [userPref]);

  React.useEffect(() => {
    const onResize = () => {
      setForcedByViewport(window.innerWidth < AUTO_COLLAPSE_BREAKPOINT);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Cmd+Ctrl+S on macOS, Ctrl+Alt+S elsewhere. We accept either combo
      // since the app currently ships on macOS but the binding is robust
      // to future Linux/Windows builds.
      const macCombo = e.metaKey && e.ctrlKey && e.key.toLowerCase() === "s";
      const otherCombo = e.ctrlKey && e.altKey && e.key.toLowerCase() === "s";
      if (!macCombo && !otherCombo) return;
      e.preventDefault();
      setUserPref((cur) => !cur);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const collapsed = userPref || forcedByViewport;
  const toggle = React.useCallback(() => setUserPref((c) => !c), []);

  return { collapsed, toggle, forcedByViewport };
}
