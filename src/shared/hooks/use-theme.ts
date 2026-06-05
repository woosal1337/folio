import * as React from "react";

export type Theme = "light" | "dark";

const STORAGE_KEY = "folio-theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.classList.toggle("dark", theme === "dark");
}

function systemTheme(): Theme {
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

function storedTheme(): Theme | null {
  if (typeof window === "undefined") return null;
  const v = window.localStorage.getItem(STORAGE_KEY);
  return v === "light" || v === "dark" ? v : null;
}

function readInitial(): Theme {
  return storedTheme() ?? systemTheme();
}

export function useTheme() {
  const [theme, setThemeState] = React.useState<Theme>(() => readInitial());

  React.useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  React.useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia(DARK_QUERY);
    const onChange = () => {
      if (storedTheme() === null) setThemeState(mq.matches ? "dark" : "light");
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  const setTheme = React.useCallback((t: Theme) => {
    window.localStorage.setItem(STORAGE_KEY, t);
    setThemeState(t);
  }, []);
  const toggle = React.useCallback(() => {
    setThemeState((cur) => {
      const next = cur === "light" ? "dark" : "light";
      window.localStorage.setItem(STORAGE_KEY, next);
      return next;
    });
  }, []);
  return { theme, setTheme, toggle };
}

export function applyInitialTheme() {
  applyTheme(readInitial());
}
