import * as React from "react";
import { useNavigate } from "react-router-dom";

import { useRecording } from "@/shared/stores/recording-store";

/**
 * GET-144 — the one-click "Take Notes" flow.
 *
 * A single action that opens the meeting note (the Record route) and
 * starts mic + system capture. The three entry points — the meeting HUD
 * (GET-143), the menu-bar tray, and Cmd-R — all converge here.
 *
 * Idempotent on an active session: if capture is already running (or
 * mid-start) we just focus the note rather than starting a second
 * recording. The backend also rejects a double-start, but guarding here
 * keeps the UX quiet (no error toast) and is the seam the start/stop
 * race fix (GET-149) builds on.
 *
 * Must be called from inside the router so `navigate` is available.
 */
export function useTakeNotes(): () => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);

  return React.useCallback(() => {
    navigate("/record");
    const { recording, busy } = useRecording.getState();
    if (!recording && !busy) void start();
  }, [navigate, start]);
}
