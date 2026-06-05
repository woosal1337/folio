import * as React from "react";

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
