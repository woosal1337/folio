import * as React from "react";

import type { Theme } from "@/lib/types";

const STORAGE_KEY = "attune-theme";

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.classList.toggle("dark", theme === "dark");
}

function readInitial(): Theme {
  if (typeof window === "undefined") return "light";
  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return "light";
}

export function useTheme() {
  const [theme, setThemeState] = React.useState<Theme>(() => readInitial());

  React.useEffect(() => {
    applyTheme(theme);
    window.localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  const setTheme = React.useCallback((t: Theme) => setThemeState(t), []);
  const toggle = React.useCallback(
    () => setThemeState((cur) => (cur === "light" ? "dark" : "light")),
    []
  );
  return { theme, setTheme, toggle };
}

/** Read + apply the saved theme on first paint, before React mounts, so the
 *  app doesn't flash the wrong palette. Call from main.tsx. */
export function applyInitialTheme() {
  applyTheme(readInitial());
}
