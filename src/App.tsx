import * as React from "react";
import { HashRouter, Route, Routes, Navigate, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";

import { Sidebar } from "@/chrome/sidebar";
import { DragStrip } from "@/chrome/drag-strip";
import { JobStrip } from "@/chrome/job-strip";
import { CloudCostConfirmDialog } from "@/chrome/cloud-cost-confirm-dialog";
import { ConfirmDeleteDialog } from "@/chrome/confirm-delete-dialog";
import { ContextMenu } from "@/chrome/context-menu";
import { DeepLinkHandler } from "@/chrome/deep-link-handler";
import { EntryPointBridge } from "@/chrome/entry-points";
import { GlobalShortcuts } from "@/chrome/global-shortcuts";
import { CheatsheetOverlay } from "@/chrome/cheatsheet-overlay";
import { CommandPalette } from "@/chrome/command-palette";
import { verbSource } from "@/shared/lib/command-palette";

const Home = React.lazy(() => import("@/features/home/route"));
const Chat = React.lazy(() => import("@/features/chat/route"));
const MeetingHud = React.lazy(() => import("@/features/meeting-hud/route"));
const RecordingBar = React.lazy(() => import("@/features/recording-bar/route"));
const FirstRunConductor = React.lazy(() =>
  import("@/features/onboarding/first-run").then((m) => ({
    default: m.FirstRunConductor,
  }))
);
const Library = React.lazy(() => import("@/features/library/route"));
const Editor = React.lazy(() => import("@/features/editor/route"));
const Account = React.lazy(() => import("@/features/account/route"));
const Tasks = React.lazy(() => import("@/features/tasks/route"));
const PreferencesWindow = React.lazy(
  () => import("@/features/preferences-window/route")
);
const MemoryRoute = React.lazy(() => import("@/features/memory/route"));
const StatsRoute = React.lazy(() => import("@/features/stats/route"));
const SettingsModal = React.lazy(() =>
  import("@/features/settings/route").then((m) => ({ default: m.SettingsModal }))
);
import { ErrorBoundary } from "@/error-boundary";
import { useWindowDoubleClick, useWindowDrag } from "@/shared/hooks/use-window-drag";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import {
  MEETING_HUD_WINDOW_LABEL,
  RECORDING_BAR_WINDOW_LABEL,
  currentWindowLabel,
  onRecordingBarPause,
  onRecordingBarResume,
  onRecordingBarStop,
  searchNoteContent,
} from "@/shared/lib/ipc";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";
import { useRecording } from "@/shared/stores/recording-store";

export default function App() {
  if (currentWindowLabel() === MEETING_HUD_WINDOW_LABEL) {
    return (
      <ErrorBoundary>
        <React.Suspense fallback={null}>
          <MeetingHud />
        </React.Suspense>
      </ErrorBoundary>
    );
  }

  if (currentWindowLabel() === RECORDING_BAR_WINDOW_LABEL) {
    return (
      <ErrorBoundary>
        <React.Suspense fallback={null}>
          <RecordingBar />
        </React.Suspense>
      </ErrorBoundary>
    );
  }
  return <MainApp />;
}

function MainApp() {
  const settingsOpen = useSettingsUiStore((s) => s.open);
  const setSettingsOpen = useSettingsUiStore((s) => s.setOpen);
  const openSettings = useSettingsUiStore((s) => s.openAt);
  const [cheatsheetOpen, setCheatsheetOpen] = React.useState(false);
  const [paletteOpen, setPaletteOpen] = React.useState(false);
  const onMouseDown = useWindowDrag();
  const onDoubleClick = useWindowDoubleClick();
  const loadSettings = useSettingsStore((s) => s.load);
  const syncRecording = useRecording((s) => s.syncFromBackend);

  React.useEffect(() => {
    loadSettings();
    void syncRecording();
  }, [loadSettings, syncRecording]);

  React.useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const wire = (
      subscribe: (h: () => void) => Promise<() => void>,
      action: () => void
    ) => {
      void subscribe(action).then((fn) => {
        if (disposed) fn();
        else unlisteners.push(fn);
      });
    };
    wire(onRecordingBarStop, () => void useRecording.getState().stop());
    wire(onRecordingBarPause, () => void useRecording.getState().pause());
    wire(onRecordingBarResume, () => void useRecording.getState().resume());
    return () => {
      disposed = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  const settingsHydrated = useSettingsStore((s) => s.settings !== null);
  const onboardingCompleted = useSettingsStore(
    (s) => s.settings?.onboarding_completed ?? false
  );
  const reloadSettings = useSettingsStore((s) => s.load);

  if (!settingsHydrated) {
    return (
      <ErrorBoundary>
        {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- Tauri drag-region root, same pattern as the main shell below. */}
        <div
          className="flex h-screen w-screen items-center justify-center bg-background text-sm text-muted-foreground"
          onMouseDown={onMouseDown}
          onDoubleClick={onDoubleClick}
        >
          Loading…
        </div>
      </ErrorBoundary>
    );
  }
  if (!onboardingCompleted) {
    return (
      <ErrorBoundary>
        {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- Tauri drag-region root, same pattern as the signed-in shell below. */}
        <div
          className="flex h-screen w-screen flex-col overflow-hidden bg-background"
          onMouseDown={onMouseDown}
          onDoubleClick={onDoubleClick}
        >
          <DragStrip />
          <main className="flex-1 overflow-y-auto">
            <React.Suspense fallback={<RouteLoading />}>
              <FirstRunConductor onFinish={() => reloadSettings()} />
            </React.Suspense>
          </main>
          <Toaster theme="system" position="bottom-right" richColors closeButton />
        </div>
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary>
      <HashRouter>
        {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- NOTE: Tauri drag-region root; the data-drag attribute opt-in inside the handler is the documented Tauri pattern. Keyboard equivalents (Cmd-R, Cmd-W, etc.) live in GlobalShortcuts. */}
        <div
          className="flex h-screen w-screen flex-col overflow-hidden bg-background"
          onMouseDown={onMouseDown}
          onDoubleClick={onDoubleClick}
        >
          <DragStrip />

          <JobStrip />
          <div className="flex flex-1 overflow-hidden">
            <Sidebar onOpenSettings={() => openSettings()} />
            <main className="flex-1 overflow-y-auto">
              <React.Suspense fallback={<RouteLoading />}>
                <Routes>
                  <Route path="/" element={<Home />} />
                  <Route path="/chat" element={<Chat />} />

                  <Route path="/record" element={<Navigate to="/" replace />} />
                  <Route path="/library" element={<Library />} />
                  <Route path="/editor" element={<Navigate to="/library" replace />} />
                  <Route path="/editor/:label" element={<Editor />} />

                  <Route path="/inbox" element={<Navigate to="/" replace />} />
                  <Route path="/account" element={<Account />} />
                  <Route path="/preferences-window" element={<PreferencesWindow />} />
                  <Route path="/ai" element={<Navigate to="/" replace />} />
                  <Route path="/tasks" element={<Tasks />} />
                  <Route path="/memory" element={<MemoryRoute />} />
                  <Route path="/stats" element={<StatsRoute />} />
                  <Route path="*" element={<Navigate to="/" replace />} />
                </Routes>
              </React.Suspense>
            </main>
          </div>
          <React.Suspense fallback={null}>
            <SettingsModal open={settingsOpen} onOpenChange={setSettingsOpen} />
          </React.Suspense>
          <CloudCostConfirmDialog />
          <ConfirmDeleteDialog />
          <ContextMenu />
          <DeepLinkHandler />
          <EntryPointBridge />
          <GlobalShortcuts
            onOpenCheatsheet={() => setCheatsheetOpen(true)}
            onOpenPalette={() => setPaletteOpen(true)}
          />
          <CheatsheetOverlay
            open={cheatsheetOpen}
            onClose={() => setCheatsheetOpen(false)}
          />
          <PaletteHost
            open={paletteOpen}
            onClose={() => setPaletteOpen(false)}
            onOpenPreferences={() => openSettings()}
            onOpenCheatsheet={() => setCheatsheetOpen(true)}
          />
        </div>
        <Toaster position="bottom-right" richColors closeButton />
      </HashRouter>
    </ErrorBoundary>
  );
}

function PaletteHost({
  open,
  onClose,
  onOpenPreferences,
  onOpenCheatsheet,
}: {
  open: boolean;
  onClose: () => void;
  onOpenPreferences: () => void;
  onOpenCheatsheet: () => void;
}) {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const sources = React.useMemo(
    () => [
      verbSource({
        startRecording: takeNotes,
        openChat: () => navigate("/chat"),
        openLibrary: () => navigate("/library"),
        openMemory: () => navigate("/memory"),
        openTasks: () => navigate("/tasks"),
        openPreferences: onOpenPreferences,
        openCheatsheet: onOpenCheatsheet,
      }),

      {
        kind: "recording" as const,
        load: async () => [],
        search: async (q: string) => {
          const hits = await searchNoteContent(q);
          return hits.map((h) => ({
            id: `note:${h.label}`,
            kind: "recording" as const,
            title: h.title ?? h.label,
            subtitle: h.snippet,
            action: () => navigate(`/editor/${encodeURIComponent(h.label)}`),
          }));
        },
      },
    ],
    [navigate, takeNotes, onOpenPreferences, onOpenCheatsheet]
  );
  return <CommandPalette open={open} onClose={onClose} sources={sources} />;
}

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
