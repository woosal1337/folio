/**
 * Default landing for `/` — Library when the user has any recordings,
 * Record when they're a fresh install. v2 roadmap finding R05.
 *
 * Once the first-run conductor (#001) ships, Record will become a
 * verb reached via hotkey + menu bar rather than the perpetual home
 * tab. Until then, this redirect makes the app feel meaningful from
 * the second-launch onwards.
 */

import * as React from "react";
import { Navigate } from "react-router-dom";

import { listRecordings } from "@/shared/lib/ipc";

type Resolution = "loading" | "library" | "record";

export function HomeRedirect() {
  const [resolution, setResolution] = React.useState<Resolution>("loading");

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const rs = await listRecordings();
        if (!cancelled) setResolution(rs.length > 0 ? "library" : "record");
      } catch (e) {
        console.error("HomeRedirect: listRecordings failed", e);
        if (!cancelled) setResolution("record");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (resolution === "loading") {
    // No spinner — the resolution happens in well under one frame on
    // a typical install, and flashing a spinner here would feel like
    // a startup hitch.
    return null;
  }
  return <Navigate to={`/${resolution}`} replace />;
}
