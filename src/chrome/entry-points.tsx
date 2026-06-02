import * as React from "react";
import { useNavigate } from "react-router-dom";

import {
  onMeetingTakeNotes,
  onTrayEvent,
  onStitchingStarted,
  onStitchingDone,
} from "@/shared/lib/ipc";
import { registerNavigateFn } from "@/shared/lib/navigate-bridge";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useRecording } from "@/shared/stores/recording-store";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";

const STITCHING_JOB_ID = "finalize:stitching";

/**
 * GET-144 — converges every "start a meeting note" entry point onto the
 * single take-notes flow, and wires the rest of the menu-bar tray menu.
 *
 * GET-214 — also registers the React Router `navigate` function with
 * `navigate-bridge` so external code (deep links, future attune:// URLs)
 * can call `bridgeNavigate()` without touching `window.location` or
 * `window.history`. All external entry points MUST route through
 * `bridgeNavigate`; only in-tree components may call `useNavigate()` directly.
 *
 * Entry points handled:
 *   - meeting HUD Take Notes (`meeting:take-notes`, GET-143)
 *   - tray "Start Recording" (`tray:start-recording`)
 *   - tray "Stop Recording" / "Open Library" / "Open Inbox"
 *
 * Cmd-R converges via the same {@link useTakeNotes} hook inside
 * GlobalShortcuts. Headless — renders nothing. Mounted once inside the
 * signed-in chrome so `navigate` is available under the router.
 */
export function EntryPointBridge() {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const stop = useRecording((s) => s.stop);

  // GET-214: register navigate with the bridge so external callers that arrive
  // before mount (deep links, Tauri events) can use bridgeNavigate() safely.
  React.useEffect(() => {
    registerNavigateFn(navigate);
  }, [navigate]);

  React.useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const track = (p: Promise<() => void>) =>
      void p
        .then((fn) => unlisteners.push(fn))
        .catch((e) => console.error("entry-point listener:", e));

    track(onMeetingTakeNotes(() => takeNotes()));
    track(onTrayEvent("tray:start-recording", () => takeNotes()));
    track(
      onTrayEvent("tray:stop-recording", () => {
        if (useRecording.getState().recording) void stop();
      })
    );
    track(onTrayEvent("tray:open-library", () => navigate("/library")));
    track(onTrayEvent("tray:open-inbox", () => navigate("/inbox")));

    // WAV segment stitching events (multi-part pause/resume notes).
    // Only fires when the recording had at least one pause — so the pill
    // never flashes on single-shot recordings.
    track(
      onStitchingStarted(() => {
        useJobsStore.getState().push({
          id: STITCHING_JOB_ID,
          kind: "finalize",
          label: "Stitching recording segments…",
        });
      })
    );
    track(
      onStitchingDone(() => {
        useJobsStore.getState().pop(STITCHING_JOB_ID);
      })
    );

    return () => unlisteners.forEach((fn) => fn());
  }, [navigate, takeNotes, stop]);

  return null;
}
