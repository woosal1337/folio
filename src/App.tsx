import * as React from "react";
import { HashRouter, Route, Routes, Navigate } from "react-router-dom";
import { Toaster } from "sonner";

import { Sidebar } from "@/chrome/sidebar";
import { DragStrip } from "@/chrome/drag-strip";
import { JobStrip } from "@/chrome/job-strip";
import Record from "@/features/recording/route";
import Library from "@/features/library/route";
import Editor from "@/features/editor/route";
import Tasks from "@/features/tasks/route";
import Ai from "@/features/ai/route";
import { SettingsModal } from "@/features/settings/route";
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
              <Routes>
                <Route path="/" element={<Navigate to="/record" replace />} />
                <Route path="/record" element={<Record />} />
                <Route path="/library" element={<Library />} />
                <Route path="/editor" element={<Navigate to="/library" replace />} />
                <Route path="/editor/:label" element={<Editor />} />
                <Route path="/ai" element={<Ai />} />
                <Route path="/tasks" element={<Tasks />} />
                <Route path="*" element={<Navigate to="/record" replace />} />
              </Routes>
            </main>
          </div>
          <SettingsModal open={settingsOpen} onOpenChange={setSettingsOpen} />
        </div>
        <Toaster position="bottom-right" richColors closeButton />
      </HashRouter>
    </ErrorBoundary>
  );
}
