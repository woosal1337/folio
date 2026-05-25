import * as React from "react";
import { HashRouter, Route, Routes, Navigate } from "react-router-dom";
import { Toaster } from "sonner";

import { Sidebar } from "@/chrome/sidebar";
import { DragStrip } from "@/chrome/drag-strip";
import { JobStrip } from "@/chrome/job-strip";
import { CloudCostConfirmDialog } from "@/chrome/cloud-cost-confirm-dialog";
import { DeepLinkHandler } from "@/chrome/deep-link-handler";
import { HomeRedirect } from "@/chrome/home-redirect";
// Route components are React.lazy-loaded so the Record page (the
// dock-click landing) stays inside the cold-start budget: 400ms on
// M1, 800ms on Intel per v2 finding 058. Editor, Tasks, AI, Memory,
// and the Settings modal each ride into their own chunk and only
// arrive when the user navigates to them. The static fallback below
// renders a near-empty frame so the route swap stays visually quiet.
const Record = React.lazy(() => import("@/features/recording/route"));
const Library = React.lazy(() => import("@/features/library/route"));
const Editor = React.lazy(() => import("@/features/editor/route"));
const Tasks = React.lazy(() => import("@/features/tasks/route"));
// /ai route was retired by GET-120 — the flat agent-runs page is
// subsumed by the editor's run-cards. The route still resolves to a
// redirect into /library so prior deep-links don't 404.
const MemoryRoute = React.lazy(() => import("@/features/memory/route"));
const SettingsModal = React.lazy(() =>
  import("@/features/settings/route").then((m) => ({ default: m.SettingsModal }))
);
import { ErrorBoundary } from "@/error-boundary";
import { useWindowDoubleClick, useWindowDrag } from "@/shared/hooks/use-window-drag";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";

export default function App() {
  // Settings modal open/section lives in a global store so any component
  // (sidebar button, agent-panel hints, future deep-links) can open it
  // at a specific section without prop-drilling.
  const settingsOpen = useSettingsUiStore((s) => s.open);
  const setSettingsOpen = useSettingsUiStore((s) => s.setOpen);
  const openSettings = useSettingsUiStore((s) => s.openAt);
  const onMouseDown = useWindowDrag();
  const onDoubleClick = useWindowDoubleClick();
  const loadSettings = useSettingsStore((s) => s.load);

  // Load settings once at mount. The recording store reads from this
  // cache when deciding whether to auto-transcribe after stop, so the
  // settings need to be in memory before the first stop fires.
  React.useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  return (
    <ErrorBoundary>
      <HashRouter>
        <div
          className="flex h-screen w-screen flex-col overflow-hidden bg-background"
          onMouseDown={onMouseDown}
          onDoubleClick={onDoubleClick}
        >
          {/* Window drag strip — full-width across the top, draggable via
              data-tauri-drag-region AND an explicit startDragging() handler
              so it works on every Tauri/macOS combination. */}
          <DragStrip />
          {/* In-flight job pills (transcriptions, agent runs, model
              downloads). Renders nothing when no jobs are active so the
              chrome stays out of the way during idle. */}
          <JobStrip />
          <div className="flex flex-1 overflow-hidden">
            <Sidebar onOpenSettings={() => openSettings()} />
            <main className="flex-1 overflow-y-auto">
              <React.Suspense fallback={<RouteLoading />}>
                <Routes>
                  <Route path="/" element={<HomeRedirect />} />
                  <Route path="/record" element={<Record />} />
                  <Route path="/library" element={<Library />} />
                  <Route path="/editor" element={<Navigate to="/library" replace />} />
                  <Route path="/editor/:label" element={<Editor />} />
                  <Route path="/ai" element={<Navigate to="/library" replace />} />
                  <Route path="/tasks" element={<Tasks />} />
                  <Route path="/memory" element={<MemoryRoute />} />
                  <Route path="*" element={<HomeRedirect />} />
                </Routes>
              </React.Suspense>
            </main>
          </div>
          <React.Suspense fallback={null}>
            <SettingsModal open={settingsOpen} onOpenChange={setSettingsOpen} />
          </React.Suspense>
          <CloudCostConfirmDialog />
          <DeepLinkHandler />
        </div>
        <Toaster position="bottom-right" richColors closeButton />
      </HashRouter>
    </ErrorBoundary>
  );
}

/** Quiet fallback while a route's chunk is loading. Renders nothing
 *  for the first 120ms so a fast cache hit doesn't flash a spinner,
 *  then surfaces a centred subtle hint. v2 finding 058 / GET-93. */
function RouteLoading() {
  const [showHint, setShowHint] = React.useState(false);
  React.useEffect(() => {
    const t = window.setTimeout(() => setShowHint(true), 120);
    return () => window.clearTimeout(t);
  }, []);
  return (
    <div
      className="flex h-full w-full items-center justify-center text-xs text-muted-foreground"
      role="status"
      aria-live="polite"
    >
      {showHint ? "Loading…" : null}
    </div>
  );
}
