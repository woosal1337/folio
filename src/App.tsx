import * as React from "react";
import { HashRouter, Route, Routes, Navigate, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";

import { Sidebar } from "@/chrome/sidebar";
import { DragStrip } from "@/chrome/drag-strip";
import { JobStrip } from "@/chrome/job-strip";
import { CloudCostConfirmDialog } from "@/chrome/cloud-cost-confirm-dialog";
import { DeepLinkHandler } from "@/chrome/deep-link-handler";
import { EntryPointBridge } from "@/chrome/entry-points";
import { GlobalShortcuts } from "@/chrome/global-shortcuts";
import { CheatsheetOverlay } from "@/chrome/cheatsheet-overlay";
import { CommandPalette } from "@/chrome/command-palette";
import { verbSource } from "@/shared/lib/command-palette";
// Route components are React.lazy-loaded so the Record page (the
// dock-click landing) stays inside the cold-start budget: 400ms on
// M1, 800ms on Intel per v2 finding 058. Editor, Tasks, AI, Memory,
// and the Settings modal each ride into their own chunk and only
// arrive when the user navigates to them. The static fallback below
// renders a near-empty frame so the route swap stays visually quiet.
const Home = React.lazy(() => import("@/features/home/route"));
const Chat = React.lazy(() => import("@/features/chat/route"));
const MeetingHud = React.lazy(() => import("@/features/meeting-hud/route"));
const FirstRunConductor = React.lazy(() =>
  import("@/features/onboarding/first-run").then((m) => ({
    default: m.FirstRunConductor,
  }))
);
const Library = React.lazy(() => import("@/features/library/route"));
const Editor = React.lazy(() => import("@/features/editor/route"));
const Tasks = React.lazy(() => import("@/features/tasks/route"));
const PreferencesWindow = React.lazy(
  () => import("@/features/preferences-window/route")
);
const MemoryRoute = React.lazy(() => import("@/features/memory/route"));
const SettingsModal = React.lazy(() =>
  import("@/features/settings/route").then((m) => ({ default: m.SettingsModal }))
);
import { ErrorBoundary } from "@/error-boundary";
import { useWindowDoubleClick, useWindowDrag } from "@/shared/hooks/use-window-drag";
import { useAuthStore } from "@/shared/stores/auth-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import { MEETING_HUD_WINDOW_LABEL, currentWindowLabel } from "@/shared/lib/ipc";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";
import { useRecording } from "@/shared/stores/recording-store";

export default function App() {
  // The meeting-detection HUD (GET-143) is a separate frameless window.
  // It renders a standalone surface with no sidebar, chrome, or auth
  // gate — short-circuit before any of that mounts.
  if (currentWindowLabel() === MEETING_HUD_WINDOW_LABEL) {
    return (
      <ErrorBoundary>
        <React.Suspense fallback={null}>
          <MeetingHud />
        </React.Suspense>
      </ErrorBoundary>
    );
  }
  return <MainApp />;
}

function MainApp() {
  // Settings modal open/section lives in a global store so any component
  // (sidebar button, agent-panel hints, future deep-links) can open it
  // at a specific section without prop-drilling.
  const settingsOpen = useSettingsUiStore((s) => s.open);
  const setSettingsOpen = useSettingsUiStore((s) => s.setOpen);
  const openSettings = useSettingsUiStore((s) => s.openAt);
  const [cheatsheetOpen, setCheatsheetOpen] = React.useState(false);
  const [paletteOpen, setPaletteOpen] = React.useState(false);
  const onMouseDown = useWindowDrag();
  const onDoubleClick = useWindowDoubleClick();
  const loadSettings = useSettingsStore((s) => s.load);
  const hydrateAuth = useAuthStore((s) => s.hydrate);
  const syncRecording = useRecording((s) => s.syncFromBackend);

  // Load settings once at mount. The recording store reads from this
  // cache when deciding whether to auto-transcribe after stop, so the
  // settings need to be in memory before the first stop fires.
  // Also probe the backend for an in-progress capture (GET-155): with
  // the Record screen gone, this is the one place that re-adopts a
  // recording after a reload so the in-note dock reflects it.
  React.useEffect(() => {
    loadSettings();
    void hydrateAuth();
    void syncRecording();
  }, [loadSettings, hydrateAuth, syncRecording]);

  // Force-login + post-signup setup: the sidebar + every route is
  // invisible until BOTH the user holds a valid Keychain session AND
  // `onboarding_completed` is true on disk. The conductor takes the
  // full window for either case — signed-out users sign in; freshly-
  // signed-in users finish workspace setup (EventKit → workspace
  // name → bucket → invite teammates → transcriber → "I'm ready").
  // Only after the conductor flips `onboarding_completed` does the
  // main chrome render.
  const authHydrated = useAuthStore((s) => s.hydrated);
  const signedIn = useAuthStore((s) => s.signedIn);
  const settingsHydrated = useSettingsStore((s) => s.settings !== null);
  const onboardingCompleted = useSettingsStore(
    (s) => s.settings?.onboarding_completed ?? false
  );
  const reloadSettings = useSettingsStore((s) => s.load);

  if (!authHydrated || !settingsHydrated) {
    return (
      <ErrorBoundary>
        {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- Tauri drag-region root, same pattern as the signed-in shell below. */}
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
  if (!signedIn || !onboardingCompleted) {
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
                  <Route path="/" element={<Home />} />
                  <Route path="/chat" element={<Chat />} />
                  {/* /record retired (GET-155): recording is a note dock now */}
                  <Route path="/record" element={<Navigate to="/" replace />} />
                  <Route path="/library" element={<Library />} />
                  <Route path="/editor" element={<Navigate to="/library" replace />} />
                  <Route path="/editor/:label" element={<Editor />} />
                  {/* Inbox retired (GET-157): actions live in Tasks, memories
                      in Memory, runs in the note. /inbox + /ai redirect home. */}
                  <Route path="/inbox" element={<Navigate to="/" replace />} />
                  <Route path="/preferences-window" element={<PreferencesWindow />} />
                  <Route path="/ai" element={<Navigate to="/" replace />} />
                  <Route path="/tasks" element={<Tasks />} />
                  <Route path="/memory" element={<MemoryRoute />} />
                  <Route path="*" element={<Navigate to="/" replace />} />
                </Routes>
              </React.Suspense>
            </main>
          </div>
          <React.Suspense fallback={null}>
            <SettingsModal open={settingsOpen} onOpenChange={setSettingsOpen} />
          </React.Suspense>
          <CloudCostConfirmDialog />
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

/**
 * Build the Cmd-K palette's source list. Wraps the verb source from
 * the catalogue + (in a follow-up) per-data sources for recordings /
 * tasks / memories. Keeping this in a sub-component lets useNavigate
 * be called inside HashRouter while App.tsx itself doesn't need it.
 */
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
    ],
    [navigate, takeNotes, onOpenPreferences, onOpenCheatsheet]
  );
  return <CommandPalette open={open} onClose={onClose} sources={sources} />;
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
